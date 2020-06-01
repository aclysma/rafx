use crate::pipeline_description as dsc;
use renderer_base::slab::{RawSlab, RawSlabKey};
use super::RegisteredDescriptorSet;
use super::{
    SlabKeyDescriptorSetWriteSet, SlabKeyDescriptorSetWriteBuffer,
    DescriptorSetPoolRequiredBufferInfo, MAX_DESCRIPTORS_PER_POOL, MAX_FRAMES_IN_FLIGHT_PLUS_1,
    MAX_FRAMES_IN_FLIGHT, DescriptorSetElementKey, FrameInFlightIndex, DescriptorSetArc,
    DescriptorSetWriteSet,
};
use crate::resource_managers::resource_lookup::{DescriptorSetLayoutResource, ResourceArc};
use renderer_shell_vulkan::{VkDescriptorPoolAllocator, VkResourceDropSink, VkBuffer, VkDeviceContext};
use crossbeam_channel::{Receiver, Sender};
use std::mem::ManuallyDrop;
use ash::vk;
use ash::version::DeviceV1_0;
use ash::prelude::VkResult;
use super::RegisteredDescriptorSetPoolChunk;

pub(super) struct RegisteredDescriptorSetPool {
    //descriptor_set_layout_def: dsc::DescriptorSetLayout,
    pub(super) slab: RawSlab<RegisteredDescriptorSet>,
    //pending_allocations: Vec<DescriptorSetWrite>,
    drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>,
    drop_rx: Receiver<RawSlabKey<RegisteredDescriptorSet>>,
    pub(super) write_set_tx: Sender<SlabKeyDescriptorSetWriteSet>,
    write_set_rx: Receiver<SlabKeyDescriptorSetWriteSet>,
    //pub(super) write_buffer_tx: Sender<SlabKeyDescriptorSetWriteBuffer>,
    //write_buffer_rx: Receiver<SlabKeyDescriptorSetWriteBuffer>,
    descriptor_pool_allocator: VkDescriptorPoolAllocator,
    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,

    buffer_drop_sink: VkResourceDropSink<ManuallyDrop<VkBuffer>>,

    //descriptor_set_layout_def: dsc::DescriptorSetLayout,
    buffer_infos: Vec<DescriptorSetPoolRequiredBufferInfo>,

    chunks: Vec<RegisteredDescriptorSetPoolChunk>,
}

impl RegisteredDescriptorSetPool {
    pub fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();
        let (write_set_tx, write_set_rx) = crossbeam_channel::unbounded();
        //let (write_buffer_tx, write_buffer_rx) = crossbeam_channel::unbounded();

        //
        // This is a little gross but it creates the pool sizes required for the
        // DescriptorPoolCreateInfo passed into create_descriptor_pool. Do it here once instead of
        // in the allocator callback
        //
        let mut descriptor_counts = vec![0; dsc::DescriptorType::count()];
        for desc in &descriptor_set_layout_def.descriptor_set_layout_bindings {
            let ty: vk::DescriptorType = desc.descriptor_type.into();
            descriptor_counts[ty.as_raw() as usize] +=
                MAX_DESCRIPTORS_PER_POOL * (MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32);
        }

        let mut pool_sizes = Vec::with_capacity(dsc::DescriptorType::count());
        for (descriptor_type, count) in descriptor_counts.into_iter().enumerate() {
            if count > 0 {
                let pool_size = vk::DescriptorPoolSize::builder()
                    .descriptor_count(count as u32)
                    .ty(vk::DescriptorType::from_raw(descriptor_type as i32))
                    .build();
                pool_sizes.push(pool_size);
            }
        }

        // The allocator will produce descriptor sets as needed and destroy them after waiting a few
        // frames for them to finish any submits that reference them
        let descriptor_pool_allocator = VkDescriptorPoolAllocator::new(
            MAX_FRAMES_IN_FLIGHT as u32,
            MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32,
            move |device| {
                let pool_builder = vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(MAX_DESCRIPTORS_PER_POOL * MAX_FRAMES_IN_FLIGHT_PLUS_1 as u32)
                    .pool_sizes(&pool_sizes);

                unsafe { device.create_descriptor_pool(&*pool_builder, None) }
            },
        );

        let mut buffer_infos = Vec::new();
        for binding in &descriptor_set_layout_def.descriptor_set_layout_bindings {
            if let Some(per_descriptor_size) = binding.internal_buffer_per_descriptor_size {
                //TODO: 256 is the max allowed by the vulkan spec but we could improve this by using the
                // actual hardware value given by device limits
                let required_alignment =
                    device_context.limits().min_uniform_buffer_offset_alignment as u32;
                let per_descriptor_stride =
                    renderer_shell_vulkan::util::round_size_up_to_alignment_u32(
                        per_descriptor_size,
                        required_alignment,
                    );

                buffer_infos.push(DescriptorSetPoolRequiredBufferInfo {
                    per_descriptor_size,
                    per_descriptor_stride,
                    descriptor_type: binding.descriptor_type,
                    dst_element: DescriptorSetElementKey {
                        dst_binding: binding.binding,
                    },
                })
            }
        }

        RegisteredDescriptorSetPool {
            slab: RawSlab::with_capacity(MAX_DESCRIPTORS_PER_POOL),
            drop_tx,
            drop_rx,
            write_set_tx,
            write_set_rx,
            //write_buffer_tx,
            //write_buffer_rx,
            descriptor_pool_allocator,
            descriptor_set_layout,
            chunks: Default::default(),
            buffer_infos,
            buffer_drop_sink: VkResourceDropSink::new(MAX_FRAMES_IN_FLIGHT as u32),
        }
    }

    pub fn insert(
        &mut self,
        device_context: &VkDeviceContext,
        write_set: DescriptorSetWriteSet,
        frame_in_flight_index: FrameInFlightIndex,
    ) -> VkResult<DescriptorSetArc> {
        let registered_set = RegisteredDescriptorSet {
            // Don't have anything to store yet
            //write_set: write_set.clone()
        };

        // Use the slab allocator to find an unused index, determine the chunk index from that
        let slab_key = self.slab.allocate(registered_set);
        let chunk_index = (slab_key.index() / MAX_DESCRIPTORS_PER_POOL) as usize;

        // Add more chunks if necessary
        while chunk_index as usize >= self.chunks.len() {
            self.chunks.push(RegisteredDescriptorSetPoolChunk::new(
                device_context,
                &self.buffer_infos,
                self.descriptor_set_layout.get_raw().descriptor_set_layout,
                &mut self.descriptor_pool_allocator,
            )?);
        }

        // Insert the write into the chunk, it will be applied when update() is next called on it
        let descriptor_sets_per_frame =
            self.chunks[chunk_index].schedule_write_set(slab_key, write_set, frame_in_flight_index);

        // Return the ref-counted descriptor set
        let descriptor_set =
            DescriptorSetArc::new(slab_key, descriptor_sets_per_frame, self.drop_tx.clone());
        Ok(descriptor_set)
    }

    pub fn schedule_changes(
        &mut self,
        device_context: &VkDeviceContext,
        frame_in_flight_index: FrameInFlightIndex,
    ) {
        for write in self.write_set_rx.try_iter() {
            log::trace!(
                "Received a set write for frame in flight index {}",
                frame_in_flight_index
            );
            let chunk_index = write.slab_key.index() / MAX_DESCRIPTORS_PER_POOL;
            self.chunks[chunk_index as usize].schedule_write_set(
                write.slab_key,
                write.write_set,
                frame_in_flight_index,
            );
        }
/*
        for write in self.write_buffer_rx.try_iter() {
            log::trace!(
                "Received a buffer write for frame in flight index {}",
                frame_in_flight_index
            );
            let chunk_index = write.slab_key.index() / MAX_DESCRIPTORS_PER_POOL;
            self.chunks[chunk_index as usize].schedule_write_buffer(
                write.slab_key,
                write.write_buffer,
                frame_in_flight_index,
            );
        }
        */
    }

    pub fn flush_changes(
        &mut self,
        device_context: &VkDeviceContext,
        frame_in_flight_index: FrameInFlightIndex,
    ) {
        // Route messages that indicate a dropped descriptor set to the chunk that owns it
        for dropped in self.drop_rx.try_iter() {
            self.slab.free(dropped);
        }

        // Commit pending writes/removes, rotate to the descriptor set for the next frame
        for chunk in &mut self.chunks {
            chunk.update(device_context, frame_in_flight_index);
        }

        self.descriptor_pool_allocator
            .update(device_context.device());
    }

    pub fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        for chunk in &mut self.chunks {
            chunk.destroy(
                &mut self.descriptor_pool_allocator,
                &mut self.buffer_drop_sink,
            );
        }

        self.descriptor_pool_allocator
            .destroy(device_context.device());
        self.buffer_drop_sink.destroy(&device_context);
        self.chunks.clear();
    }
}
