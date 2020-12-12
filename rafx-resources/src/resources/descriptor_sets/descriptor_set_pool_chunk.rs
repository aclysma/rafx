use super::DescriptorSetWriteElementBufferData;
use super::{
    DescriptorLayoutBufferSet, DescriptorSetElementKey, DescriptorSetPoolRequiredBufferInfo,
    DescriptorSetWriteSet, ManagedDescriptorSet, MAX_DESCRIPTORS_PER_POOL,
};
use crate::vk_description as dsc;
use arrayvec::ArrayVec;
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk;
use fnv::FnvHashMap;
use rafx_base::slab::RawSlabKey;
use rafx_shell_vulkan::{VkBuffer, VkDescriptorPoolAllocator, VkDeviceContext, VkResourceDropSink};
use std::collections::VecDeque;
use std::mem::ManuallyDrop;

// A write to the descriptors within a single descriptor set that has been scheduled (i.e. will occur
// over the next MAX_FRAMES_IN_FLIGHT_PLUS_1 frames
#[derive(Debug)]
struct PendingDescriptorSetWriteSet {
    slab_key: RawSlabKey<ManagedDescriptorSet>,
    write_set: DescriptorSetWriteSet,
}

//
// A single chunk within a pool. This allows us to create MAX_DESCRIPTORS_PER_POOL * MAX_FRAMES_IN_FLIGHT_PLUS_1
// descriptors for a single descriptor set layout
//
pub(super) struct ManagedDescriptorSetPoolChunk {
    // We only need the layout for logging
    descriptor_set_layout: vk::DescriptorSetLayout,

    // The pool holding all descriptors in this chunk
    pool: vk::DescriptorPool,

    // The descriptors
    descriptor_sets: Vec<vk::DescriptorSet>,

    // The buffers that back the descriptor sets
    buffers: DescriptorLayoutBufferSet,

    // The writes that have been scheduled to occur over the next MAX_FRAMES_IN_FLIGHT_PLUS_1 frames. This
    // ensures that each frame's descriptor sets/buffers are appropriately updated
    pending_set_writes: VecDeque<PendingDescriptorSetWriteSet>,
}

impl ManagedDescriptorSetPoolChunk {
    #[profiling::function]
    pub(super) fn new(
        device_context: &VkDeviceContext,
        buffer_info: &[DescriptorSetPoolRequiredBufferInfo],
        descriptor_set_layout: vk::DescriptorSetLayout,
        allocator: &mut VkDescriptorPoolAllocator,
    ) -> VkResult<Self> {
        let pool = allocator.allocate_pool(device_context.device())?;

        // This structure describes how the descriptor sets will be allocated.
        let descriptor_set_layouts = [descriptor_set_layout; MAX_DESCRIPTORS_PER_POOL as usize];

        // We need to allocate the full set once per frame in flight, plus one frame not-in-flight
        // that we can modify
        let set_create_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&descriptor_set_layouts);

        let descriptor_sets = unsafe {
            device_context
                .device()
                .allocate_descriptor_sets(&*set_create_info)?
        };

        // Now allocate all the buffers that act as backing-stores for descriptor sets
        let buffers = DescriptorLayoutBufferSet::new(device_context, buffer_info)?;

        // There is some trickiness here, vk::WriteDescriptorSet will hold a pointer to vk::DescriptorBufferInfos
        // that have been pushed into `write_descriptor_buffer_infos`. We don't want to use a Vec
        // since it can realloc and invalidate the pointers.
        const DESCRIPTOR_COUNT: usize = MAX_DESCRIPTORS_PER_POOL as usize;
        const MAX_BUFFER_SETS_PER_DESCRIPTOR_SET: usize = 8;
        let mut write_descriptor_buffer_infos: ArrayVec<
            [_; DESCRIPTOR_COUNT * MAX_BUFFER_SETS_PER_DESCRIPTOR_SET],
        > = ArrayVec::new();
        let mut descriptor_writes = Vec::new();

        // If we trip this, we have more buffers in a single descriptor set than expected. It's not
        // a problem to bump this, just increases a stack allocation a bit.
        assert!(buffers.buffer_sets.len() < MAX_BUFFER_SETS_PER_DESCRIPTOR_SET);

        // For every binding/buffer set
        for (binding_key, binding_buffers) in &buffers.buffer_sets {
            // For every descriptor
            let mut offset = 0;
            for descriptor_set in &descriptor_sets {
                let buffer_info = [vk::DescriptorBufferInfo::builder()
                    .buffer(binding_buffers.buffer.buffer())
                    .range(binding_buffers.buffer_info.per_descriptor_size as u64)
                    .offset(offset)
                    .build()];

                // The array of buffer infos has to persist until all WriteDescriptorSet are
                // built and written
                write_descriptor_buffer_infos.push(buffer_info);

                let descriptor_set_write = vk::WriteDescriptorSet::builder()
                    .dst_set(*descriptor_set)
                    .dst_binding(binding_key.dst_binding)
                    .dst_array_element(0) // this is zero because we're binding an array of elements
                    .descriptor_type(binding_buffers.buffer_info.descriptor_type.into())
                    .buffer_info(&*write_descriptor_buffer_infos.last().unwrap())
                    .build();

                descriptor_writes.push(descriptor_set_write);

                offset += binding_buffers.buffer_info.per_descriptor_stride as u64;
            }
        }

        unsafe {
            device_context
                .device()
                .update_descriptor_sets(&descriptor_writes, &[]);
        }

        Ok(ManagedDescriptorSetPoolChunk {
            descriptor_set_layout,
            pool,
            descriptor_sets,
            pending_set_writes: Default::default(),
            buffers,
        })
    }

    pub(super) fn destroy(
        &mut self,
        pool_allocator: &mut VkDescriptorPoolAllocator,
        buffer_drop_sink: &mut VkResourceDropSink<ManuallyDrop<VkBuffer>>,
    ) {
        pool_allocator.retire_pool(self.pool);
        for (_, buffer_set) in self.buffers.buffer_sets.drain() {
            buffer_drop_sink.retire(buffer_set.buffer);
        }
    }

    pub(super) fn schedule_write_set(
        &mut self,
        slab_key: RawSlabKey<ManagedDescriptorSet>,
        write_set: DescriptorSetWriteSet,
    ) -> vk::DescriptorSet {
        log::trace!(
            "Schedule a write for descriptor set {:?} on layout {:?}",
            slab_key,
            self.descriptor_set_layout
        );
        //log::trace!("{:#?}", write_set);

        // Use frame_in_flight_index for the live_until_frame because every update, we immediately
        // increment the frame and *then* do updates. So by setting it to the pre-next-update
        // frame_in_flight_index, this will make the write stick around for this and the next
        // MAX_FRAMES_IN_FLIGHT frames
        let pending_write = PendingDescriptorSetWriteSet {
            slab_key,
            write_set,
        };

        //TODO: Consider pushing these into a hashmap for the frame and let the pending write array
        // be a list of hashmaps
        self.pending_set_writes.push_back(pending_write);

        let descriptor_index = slab_key.index() % MAX_DESCRIPTORS_PER_POOL;
        self.descriptor_sets[descriptor_index as usize]
    }

    #[profiling::function]
    pub(super) fn update(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        // This function is a bit tricky unfortunately. We need to build a list of vk::WriteDescriptorSet
        // but this struct has a pointer to data in image_infos/buffer_infos. To deal with this, we
        // need to push the temporary lists of these infos into these lists. This way they don't
        // drop out of scope while we are using them. Ash does do some lifetime tracking, but once
        // you call build() it completely trusts that any pointers it holds will stay valid. So
        // while these lists are mutable to allow pushing data in, the Vecs inside must not be modified.
        let mut vk_image_infos = vec![];
        let mut vk_buffer_infos = vec![];
        //let mut vk_buffer_infos = vec![];

        #[derive(PartialEq, Eq, Hash, Debug)]
        struct SlabElementKey(RawSlabKey<ManagedDescriptorSet>, DescriptorSetElementKey);

        // Flatten the vec of hash maps into a single hashmap. This eliminates any duplicate
        // sets with the most recent set taking precedence
        let mut all_set_writes = FnvHashMap::default();
        for pending_write in &self.pending_set_writes {
            for (key, value) in &pending_write.write_set.elements {
                all_set_writes.insert(SlabElementKey(pending_write.slab_key, *key), value);
            }
        }

        let mut write_builders = vec![];
        for (key, element) in all_set_writes {
            let slab_key = key.0;
            let element_key = key.1;

            //log::trace!("{:#?}", element);

            let descriptor_set_index = slab_key.index() % MAX_DESCRIPTORS_PER_POOL;
            let descriptor_set = self.descriptor_sets[descriptor_set_index as usize];

            log::trace!(
                "Process descriptor set pending_write for {:?} {:?}. layout {:?} set {:?}",
                slab_key,
                element_key,
                self.descriptor_set_layout,
                descriptor_set
            );

            let mut builder = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(element_key.dst_binding)
                .dst_array_element(0) // This is zero because we are binding an array of elements
                .descriptor_type(element.descriptor_type.into());

            //TODO: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkWriteDescriptorSet.html has
            // info on what fields need to be set based on descriptor type
            let mut image_infos = Vec::with_capacity(element.image_info.len());
            if !element.image_info.is_empty() {
                for image_info in &element.image_info {
                    if element.has_immutable_sampler
                        && element.descriptor_type == dsc::DescriptorType::Sampler
                    {
                        // Skip any sampler bindings if the binding is populated with an immutable sampler
                        continue;
                    }

                    if image_info.sampler.is_none() && image_info.image_view.is_none() {
                        // Don't bind anything that has both a null sampler and image_view
                        continue;
                    }

                    let mut image_info_builder = vk::DescriptorImageInfo::builder();
                    image_info_builder =
                        image_info_builder.image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                    if let Some(image_view) = &image_info.image_view {
                        image_info_builder = image_info_builder.image_view(image_view.get_raw());
                    }

                    // Skip adding samplers if the binding is populated with an immutable sampler
                    // (this case is hit when using CombinedImageSampler)
                    if !element.has_immutable_sampler {
                        if let Some(sampler) = &image_info.sampler {
                            image_info_builder =
                                image_info_builder.sampler(sampler.get_raw().sampler);
                        }
                    }

                    image_infos.push(image_info_builder.build());
                }

                builder = builder.image_info(&image_infos);
            }

            let mut buffer_infos = Vec::with_capacity(element.buffer_info.len());
            if !element.buffer_info.is_empty() {
                for buffer_info in &element.buffer_info {
                    if let Some(buffer_info) = &buffer_info.buffer {
                        match buffer_info {
                            DescriptorSetWriteElementBufferData::BufferRef(buffer) => {
                                let buffer_info_builder = vk::DescriptorBufferInfo::builder()
                                    .buffer(buffer.buffer.get_raw().buffer.buffer)
                                    .offset(buffer.offset)
                                    .range(buffer.size);

                                buffer_infos.push(buffer_info_builder.build());
                            }
                            DescriptorSetWriteElementBufferData::Data(data) => {
                                //TODO: Rebind the buffer if we are no longer bound to the internal buffer, or at
                                // least fail
                                // Failing here means that we're trying to write to a descriptor's internal buffer
                                // but the binding was not configured to enabled internal buffering
                                let buffer =
                                    self.buffers.buffer_sets.get_mut(&element_key).unwrap();
                                //assert!(data.len() as u32 <= buffer.buffer_info.per_descriptor_size);
                                if data.len() as u32 > buffer.buffer_info.per_descriptor_size {
                                    panic!(
                                        "Wrote {} bytes to a descriptor set buffer that holds {} bytes layout: {:?}",
                                        data.len(),
                                        buffer.buffer_info.per_descriptor_size,
                                        self.descriptor_set_layout
                                    );
                                }

                                if data.len() as u32 != buffer.buffer_info.per_descriptor_size {
                                    log::warn!(
                                        "Wrote {} bytes to a descriptor set buffer that holds {} bytes layout: {:?}",
                                        data.len(),
                                        buffer.buffer_info.per_descriptor_size,
                                        self.descriptor_set_layout
                                    );
                                }

                                let descriptor_set_index =
                                    slab_key.index() % MAX_DESCRIPTORS_PER_POOL;
                                let offset =
                                    buffer.buffer_info.per_descriptor_stride * descriptor_set_index;

                                let buffer = &mut buffer.buffer;

                                log::trace!(
                                    "Writing {} bytes to internal buffer to set {} at offset {}",
                                    data.len(),
                                    descriptor_set_index,
                                    offset
                                );
                                buffer
                                    .write_to_host_visible_buffer_with_offset(&data, offset as u64)
                                    .unwrap();
                            }
                        }
                    }
                }

                builder = builder.buffer_info(&buffer_infos);
            }

            //TODO: DIRTY HACK
            if builder.descriptor_count == 0 {
                continue;
            }

            write_builders.push(builder.build());
            vk_image_infos.push(image_infos);
            vk_buffer_infos.push(buffer_infos);
        }

        if !write_builders.is_empty() {
            unsafe {
                profiling::scope!("Device::update_descriptor_set");
                device_context
                    .device()
                    .update_descriptor_sets(&write_builders, &[]);
            }
        }

        self.pending_set_writes.clear();
    }
}
