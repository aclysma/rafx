use super::resource_lookup::ResourceArc;
use ash::vk;
use renderer_base::slab::{RawSlabKey, RawSlab};
use crossbeam_channel::{Sender, Receiver};
use std::fmt::Formatter;
use std::sync::Arc;
use std::collections::VecDeque;
use renderer_shell_vulkan::{VkDeviceContext, VkDescriptorPoolAllocator, VkBuffer, VkResourceDropSink};
use ash::prelude::VkResult;
use fnv::{FnvHashMap, FnvHashSet};
use super::ResourceHash;
use crate::pipeline_description as dsc;
use ash::version::DeviceV1_0;
use crate::resource_managers::ResourceManager;
use crate::pipeline::pipeline::{DescriptorSetLayoutWithSlotName, MaterialInstanceSlotAssignment, MaterialInstanceAsset};
//use crate::upload::InProgressUploadPollResult::Pending;
use crate::resource_managers::asset_lookup::{SlotNameLookup, LoadedAssetLookupSet, LoadedMaterialPass, LoadedMaterialInstance, LoadedMaterial};
use atelier_assets::loader::handle::AssetHandle;
use crate::resource_managers::resource_lookup::{DescriptorSetLayoutResource, ImageViewResource, ResourceLookupSet};
use crate::pipeline_description::{DescriptorType, DescriptorSetLayoutBinding};
use std::mem::ManuallyDrop;
use arrayvec::ArrayVec;

//
// These represent a write update that can be applied to a descriptor set in a pool
//
#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementImage {
    pub sampler: Option<ResourceArc<vk::Sampler>>,
    pub image_view: Option<ResourceArc<ImageViewResource>>,
    // For now going to assume layout is always ShaderReadOnlyOptimal
    //pub image_info: vk::DescriptorImageInfo,
}

// impl DescriptorSetWriteImage {
//     pub fn new() -> Self {
//         let mut return_value = DescriptorSetWriteImage {
//             sampler: None,
//             image_view: None,
//             //image_info: Default::default()
//         };
//
//         //return_value.image_info.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
//         return_value
//     }
//
//     pub fn set_sampler(&mut self, sampler: ResourceArc<vk::Sampler>) {
//         self.image_info.sampler = sampler.get_raw();
//         self.sampler = Some(sampler);
//     }
//
//     pub fn set_image_view(&mut self, image_view: ResourceArc<ImageViewResource>) {
//         self.image_info.image_view = image_view.get_raw();
//         self.image_view = Some(image_view);
//     }
// }

#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementBuffer {
    pub buffer: Option<ResourceArc<vk::Buffer>>,
    // For now going to assume offset 0 and range of everything
    //pub buffer_info: vk::DescriptorBufferInfo,
}

// impl DescriptorSetWriteBuffer {
//     pub fn new(buffer: ResourceArc<vk::Buffer>) -> Self {
//         unimplemented!();
//         // let mut return_value = DescriptorSetWriteImage {
//         //     buffer: None,
//         //     buffer_info: Default::default()
//         // };
//         //
//         // return_value.image_info.image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
//         // return_value
//     }
// }

#[derive(Debug, Clone)]
pub struct DescriptorSetElementWrite {
    // If true, we are not permitted to modify samplers via the write
    pub has_immutable_sampler: bool,

    //pub dst_set: u32, // a pool index?
    //pub dst_layout: u32, // a hash?
    //pub dst_pool_index: u32, // a slab key?
    //pub dst_set_index: u32,

    //pub descriptor_set: DescriptorSetArc,
    //pub dst_binding: u32,
    //pub dst_array_element: u32,
    pub descriptor_type: dsc::DescriptorType,
    pub image_info: Vec<DescriptorSetWriteElementImage>,
    pub buffer_info: Vec<DescriptorSetWriteElementBuffer>,

    //pub p_texel_buffer_view: *const BufferView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DescriptorSetElementKey {
    pub dst_binding: u32,
    //pub dst_array_element: u32,
}

#[derive(Debug, Default, Clone)]
pub struct DescriptorSetWriteSet {
    pub elements: FnvHashMap<DescriptorSetElementKey, DescriptorSetElementWrite>
}

#[derive(Debug, Default, Clone)]
pub struct DescriptorSetWriteBuffer {
    pub elements: FnvHashMap<DescriptorSetElementKey, Vec<u8>>
}

#[derive(Debug)]
struct SlabKeyDescriptorSetWriteSet {
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    write_set: DescriptorSetWriteSet,
}

#[derive(Debug)]
struct SlabKeyDescriptorSetWriteBuffer {
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    write_buffer: DescriptorSetWriteBuffer,
}


struct DescriptorWriteBuilder {
    image_infos: Vec<vk::DescriptorImageInfo>,
    buffer_infos: Vec<vk::DescriptorBufferInfo>,
}

struct RegisteredDescriptorSet {
    // Anything we'd want to store per descriptor set can go here, but don't have anything yet
    //write_set: DescriptorSetWriteSet,
}

type FrameInFlightIndex = u32;

//
// Reference counting mechanism to keep descriptor sets allocated
//
struct DescriptorSetArcInner {
    // We can't cache the vk::DescriptorSet here because the pools will be cycled
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    descriptor_sets_per_frame: Vec<vk::DescriptorSet>,
    drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>,
}

impl std::fmt::Debug for DescriptorSetArcInner {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DescriptorSetArcInner")
            .field("slab_key", &self.slab_key)
            .finish()
    }
}

pub struct DescriptorSetArc {
    inner: Arc<DescriptorSetArcInner>,
}

impl DescriptorSetArc {
    fn new(
        slab_key: RawSlabKey<RegisteredDescriptorSet>,
        descriptor_sets_per_frame: Vec<vk::DescriptorSet>,
        drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>,
    ) -> Self {
        let inner = DescriptorSetArcInner {
            slab_key,
            descriptor_sets_per_frame,
            drop_tx,
        };

        DescriptorSetArc {
            inner: Arc::new(inner),
        }
    }

    pub fn get_raw(&self, resource_manager: &ResourceManager) -> vk::DescriptorSet {
        self.inner.descriptor_sets_per_frame[resource_manager.registered_descriptor_sets.frame_in_flight_index as usize]
    }
}

impl std::fmt::Debug for DescriptorSetArc {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("DescriptorSetArc")
            .field("inner", &self.inner)
            .finish()
    }
}




struct DescriptorBindingBufferSet {
    buffers: Vec<ManuallyDrop<VkBuffer>>,
    buffer_info: DescriptorSetPoolRequiredBufferInfo,
    // per_descriptor_stride: u32,
    // per_descriptor_size: u32,
    // descriptor_type: DescriptorType
}

impl DescriptorBindingBufferSet {
    fn new(device_context: &VkDeviceContext, buffer_info: &DescriptorSetPoolRequiredBufferInfo) -> VkResult<Self> {
        //This is the only one we support right now
        assert!(buffer_info.descriptor_type == DescriptorType::UniformBuffer);
        // X frames in flight, plus one not in flight that is writable
        let buffer_count = RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1;
        let mut buffers = Vec::with_capacity(RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1);
        for _ in 0..(RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1) {
            let buffer = VkBuffer::new(
                device_context,
                vk_mem::MemoryUsage::CpuToGpu,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                (buffer_info.per_descriptor_stride * RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL) as u64
            )?;

            buffers.push(ManuallyDrop::new(buffer));
        }

        Ok(DescriptorBindingBufferSet {
            buffers,
            buffer_info: buffer_info.clone()
        })
    }
}


struct DescriptorLayoutBufferSet {
    buffer_sets: FnvHashMap<DescriptorSetElementKey, DescriptorBindingBufferSet>
}

impl DescriptorLayoutBufferSet {
    fn new(device_context: &VkDeviceContext, buffer_infos: &[DescriptorSetPoolRequiredBufferInfo]) -> VkResult<Self> {
        let mut buffer_sets : FnvHashMap<DescriptorSetElementKey, DescriptorBindingBufferSet> = Default::default();
        for buffer_info in buffer_infos {
            let buffer = DescriptorBindingBufferSet::new(device_context, &buffer_info)?;
            buffer_sets.insert(buffer_info.dst_element, buffer);
        }

        Ok(DescriptorLayoutBufferSet {
            buffer_sets
        })
    }
}













#[derive(Debug)]
struct PendingDescriptorSetWriteSet {
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    write_set: DescriptorSetWriteSet,
    live_until_frame: FrameInFlightIndex,
}

#[derive(Debug)]
struct PendingDescriptorSetWriteBuffer {
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    write_buffer: DescriptorSetWriteBuffer,
    live_until_frame: FrameInFlightIndex,
}

struct RegisteredDescriptorSetPoolChunk {
    // One per frame
    //pools: Vec<vk::DescriptorPool>,
    pool: vk::DescriptorPool,
    descriptor_sets: Vec<Vec<vk::DescriptorSet>>,

    // These are stored for RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT frames so that they
    // are applied to each frame's pool
    pending_set_writes: VecDeque<PendingDescriptorSetWriteSet>,
    pending_buffer_writes: VecDeque<PendingDescriptorSetWriteBuffer>,

    buffers: DescriptorLayoutBufferSet,
}

impl RegisteredDescriptorSetPoolChunk {
    fn new(
        device_context: &VkDeviceContext,
        buffer_info: &[DescriptorSetPoolRequiredBufferInfo],
        descriptor_set_layout: vk::DescriptorSetLayout,
        allocator: &mut VkDescriptorPoolAllocator,
    ) -> VkResult<Self> {
        let pool = allocator.allocate_pool(device_context.device())?;

        // This structure describes how the descriptor sets will be allocated.
        let descriptor_set_layouts =
            [descriptor_set_layout; RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL as usize];

        // We need to allocate the full set once per frame in flight, plus one frame not-in-flight
        // that we can modify
        let mut descriptor_sets =
            Vec::with_capacity(RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1);
        for _ in 0..RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1 {
            let set_create_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&descriptor_set_layouts);

            let descriptor_sets_for_frame = unsafe {
                device_context
                    .device()
                    .allocate_descriptor_sets(&*set_create_info)?
            };
            descriptor_sets.push(descriptor_sets_for_frame);
        }

        // Now allocate all the buffers that act as backing-stores for descriptor sets
        let buffers = DescriptorLayoutBufferSet::new(device_context, buffer_info)?;


        // There is some trickiness here, vk::WriteDescriptorSet will hold a pointer to vk::DescriptorBufferInfos
        // that have been pushed into `write_descriptor_buffer_infos`. We don't want to use a Vec
        // since it can realloc and invalidate the pointers.
        const descriptor_count: usize = (RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1) * RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL as usize;
        let mut write_descriptor_buffer_infos: ArrayVec<[_;descriptor_count]> = ArrayVec::new();
        let mut descriptor_writes = Vec::new();

        // For every binding/buffer set
        for (binding_key, binding_buffers) in &buffers.buffer_sets {
            // For every per-frame buffer
            for (binding_buffer_for_frame, binding_descriptors_for_frame) in binding_buffers.buffers.iter().zip(&descriptor_sets) {
                // For every descriptor
                let mut offset = 0;
                for descriptor_set in binding_descriptors_for_frame {
                    let buffer_info = [vk::DescriptorBufferInfo::builder()
                        .buffer(binding_buffer_for_frame.buffer())
                        .range(binding_buffers.buffer_info.per_descriptor_size as u64)
                        .offset(offset)
                        .build()
                    ];

                    // The array of buffer infos has to persist until all WriteDescriptorSet are
                    // built and written
                    write_descriptor_buffer_infos.push(buffer_info);

                    let descriptor_set_write = vk::WriteDescriptorSet::builder()
                        .dst_set(*descriptor_set)
                        .dst_binding(binding_key.dst_binding)
                        //.dst_array_element(element_key.dst_array_element)
                        .dst_array_element(0)
                        .descriptor_type(binding_buffers.buffer_info.descriptor_type.into())
                        .buffer_info(&*write_descriptor_buffer_infos.last().unwrap())
                        .build();

                    descriptor_writes.push(descriptor_set_write);

                    offset += binding_buffers.buffer_info.per_descriptor_stride as u64;
                }
            }
        }

        unsafe {
            device_context.device().update_descriptor_sets(&descriptor_writes, &[]);
        }

        Ok(RegisteredDescriptorSetPoolChunk {
            pool,
            descriptor_sets,
            pending_set_writes: Default::default(),
            pending_buffer_writes: Default::default(),
            buffers,
        })
    }

    fn destroy(
        &mut self,
        pool_allocator: &mut VkDescriptorPoolAllocator,
        buffer_drop_sink: &mut VkResourceDropSink<ManuallyDrop<VkBuffer>>
    ) {
        pool_allocator.retire_pool(self.pool);
        for (key, buffer_set) in self.buffers.buffer_sets.drain() {
            for buffer in buffer_set.buffers {
                buffer_drop_sink.retire(buffer);
            }
        }
    }

    fn write_set(
        &mut self,
        slab_key: RawSlabKey<RegisteredDescriptorSet>,
        mut write_set: DescriptorSetWriteSet,
        frame_in_flight_index: FrameInFlightIndex,
    ) -> Vec<vk::DescriptorSet> {
        log::trace!("Schedule a write for descriptor set {:?}", slab_key);
        log::trace!("{:#?}", write_set);

        // Use frame_in_flight_index for the live_until_frame because every update, we immediately
        // increment the frame and *then* do updates. So by setting it to the pre-next-update
        // frame_in_flight_index, this will make the write stick around for MAX_FRAMES_IN_FLIGHT frames
        let pending_write = PendingDescriptorSetWriteSet {
            slab_key,
            write_set,
            live_until_frame: frame_in_flight_index,
        };

        //TODO: Consider pushing these into a hashmap for the frame and let the pending write array
        // be a list of hashmaps
        self.pending_set_writes.push_back(pending_write);

        let descriptor_index =
            slab_key.index() % RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL;
        self.descriptor_sets
            .iter()
            .map(|x| x[descriptor_index as usize])
            .collect()
    }

    fn write_buffer(
        &mut self,
        slab_key: RawSlabKey<RegisteredDescriptorSet>,
        mut write_buffer: DescriptorSetWriteBuffer,
        frame_in_flight_index: FrameInFlightIndex,
    ) -> Vec<vk::DescriptorSet> {
        log::trace!("Schedule a buffer write for descriptor set {:?}", slab_key);
        log::trace!("{:#?}", write_buffer);

        // Use frame_in_flight_index for the live_until_frame because every update, we immediately
        // increment the frame and *then* do updates. So by setting it to the pre-next-update
        // frame_in_flight_index, this will make the write stick around for MAX_FRAMES_IN_FLIGHT frames
        let pending_write = PendingDescriptorSetWriteBuffer {
            slab_key,
            write_buffer,
            live_until_frame: frame_in_flight_index,
        };

        //TODO: Consider pushing these into a hashmap for the frame and let the pending write array
        // be a list of hashmaps
        self.pending_buffer_writes.push_back(pending_write);

        let descriptor_index =
            slab_key.index() % RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL;
        self.descriptor_sets
            .iter()
            .map(|x| x[descriptor_index as usize])
            .collect()
    }

    fn update(
        &mut self,
        device_context: &VkDeviceContext,
        frame_in_flight_index: FrameInFlightIndex,
    ) {
        // This function is a bit tricky unfortunately. We need to build a list of vk::WriteDescriptorSet
        // but this struct has a pointer to data in image_infos/buffer_infos. To deal with this, we
        // need to push the temporary lists of these infos into these lists. This way they don't
        // drop out of scope while we are using them. Ash does do some lifetime tracking, but once
        // you call build() it completely trusts that any pointers it holds will stay valid. So
        // while these lists are mutable to allow pushing data in, the Vecs inside must not be modified.
        let mut vk_image_infos = vec![];
        //let mut vk_buffer_infos = vec![];

        #[derive(PartialEq, Eq, Hash, Debug)]
        struct SlabElementKey(RawSlabKey<RegisteredDescriptorSet>, DescriptorSetElementKey);

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

            log::trace!("Process descriptor set pending_write for {:?} {:?}", slab_key, element_key);
            log::trace!("{:#?}", element);

            let descriptor_set_index = slab_key.index()
                % RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL;
            let descriptor_set = self.descriptor_sets[frame_in_flight_index as usize]
                [descriptor_set_index as usize];

            let mut builder = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(element_key.dst_binding)
                //.dst_array_element(element_key.dst_array_element)
                .dst_array_element(0)
                .descriptor_type(element.descriptor_type.into());

            //TODO: https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkWriteDescriptorSet.html has
            // info on what fields need to be set based on descriptor type
            let mut image_infos = Vec::with_capacity(element.image_info.len());
            if !element.image_info.is_empty() {
                for image_info in &element.image_info {
                    // Skip any sampler bindings if the binding is populated with an immutable sampler
                    if element.has_immutable_sampler && element.descriptor_type == dsc::DescriptorType::Sampler {
                        continue;
                    }

                    let mut image_info_builder = vk::DescriptorImageInfo::builder();
                    image_info_builder = image_info_builder
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                    if let Some(image_view) = &image_info.image_view {
                        image_info_builder =
                            image_info_builder.image_view(image_view.get_raw().image_view);
                    }

                    // Skip adding samplers if the binding is populated with an immutable sampler
                    // (this case is hit when using CombinedImageSampler)
                    if !element.has_immutable_sampler {
                        if let Some(sampler) = &image_info.sampler {
                            image_info_builder = image_info_builder.sampler(sampler.get_raw());
                        }
                    }

                    image_infos.push(image_info_builder.build());
                }

                builder = builder.image_info(&image_infos);
            }

            //TODO: DIRTY HACK TO JUST LOAD THE IMAGE
            if image_infos.is_empty() {
                continue;
            }

            write_builders.push(builder.build());
            vk_image_infos.push(image_infos);
        }

        //DescriptorSetWrite::write_sets(self.sets[frame_in_flight_index], writes);

        //device_context.device().update_descriptor_sets()

        if !write_builders.is_empty() {
            unsafe {
                device_context
                    .device()
                    .update_descriptor_sets(&write_builders, &[]);
            }
        }

        let mut all_buffer_writes = FnvHashMap::default();
        for pending_buffer_write in &self.pending_buffer_writes {
            for (key, value) in &pending_buffer_write.write_buffer.elements {
                all_buffer_writes.insert(SlabElementKey(pending_buffer_write.slab_key, *key), value);
            }
        }

        for (key, data) in all_buffer_writes {
            let slab_key = key.0;
            let element_key = key.1;

            log::trace!("Process buffer pending_write for {:?} {:?}", slab_key, element_key);
            log::trace!("{} bytes", data.len());

            let mut buffer = self.buffers.buffer_sets.get_mut(&element_key).unwrap();
            assert!(data.len() as u32 <= buffer.buffer_info.per_descriptor_size);
            if data.len() as u32 != buffer.buffer_info.per_descriptor_size {
                log::warn!("Wrote {} bytes to a descriptor set buffer that holds {} bytes", data.len(), buffer.buffer_info.per_descriptor_size);
            }

            let descriptor_set_index = slab_key.index()
                % RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL;
            let offset = buffer.buffer_info.per_descriptor_stride * descriptor_set_index;

            let buffer = &mut buffer.buffers[frame_in_flight_index as usize];

            buffer.write_to_host_visible_buffer_with_offset(&data, offset as u64);
        }



        // Determine how many writes we can drain
        let mut pending_writes_to_drain = 0;
        for pending_write in &self.pending_set_writes {
            // If frame_in_flight_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if pending_write.live_until_frame == frame_in_flight_index {
                pending_writes_to_drain += 1;
            } else {
                break;
            }
        }

        // Drop any writes that have lived long enough to apply to the descriptor set for each frame
        self.pending_set_writes.drain(0..pending_writes_to_drain);

        // Determine how many writes we can drain
        let mut pending_writes_to_drain = 0;
        for pending_write in &self.pending_buffer_writes {
            // If frame_in_flight_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if pending_write.live_until_frame == frame_in_flight_index {
                pending_writes_to_drain += 1;
            } else {
                break;
            }
        }

        // Drop any writes that have lived long enough to apply to the descriptor set for each frame
        self.pending_buffer_writes.drain(0..pending_writes_to_drain);
    }
}

#[derive(Clone)]
struct DescriptorSetPoolRequiredBufferInfo {
    dst_element: DescriptorSetElementKey,
    descriptor_type: dsc::DescriptorType,
    per_descriptor_size: u32,
    per_descriptor_stride: u32,
}

struct RegisteredDescriptorSetPool {
    //descriptor_set_layout_def: dsc::DescriptorSetLayout,
    slab: RawSlab<RegisteredDescriptorSet>,
    //pending_allocations: Vec<DescriptorSetWrite>,
    drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>,
    drop_rx: Receiver<RawSlabKey<RegisteredDescriptorSet>>,
    write_set_tx: Sender<SlabKeyDescriptorSetWriteSet>,
    write_set_rx: Receiver<SlabKeyDescriptorSetWriteSet>,
    write_buffer_tx: Sender<SlabKeyDescriptorSetWriteBuffer>,
    write_buffer_rx: Receiver<SlabKeyDescriptorSetWriteBuffer>,
    descriptor_pool_allocator: VkDescriptorPoolAllocator,
    descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,

    buffer_drop_sink: VkResourceDropSink<ManuallyDrop<VkBuffer>>,

    //descriptor_set_layout_def: dsc::DescriptorSetLayout,
    buffer_infos: Vec<DescriptorSetPoolRequiredBufferInfo>,

    chunks: Vec<RegisteredDescriptorSetPoolChunk>,
}

impl RegisteredDescriptorSetPool {
    const MAX_DESCRIPTORS_PER_POOL: u32 = 64;
    const MAX_FRAMES_IN_FLIGHT: usize = renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT;

    pub fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> Self {
        //renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();
        let (write_set_tx, write_set_rx) = crossbeam_channel::unbounded();
        let (write_buffer_tx, write_buffer_rx) = crossbeam_channel::unbounded();

        //
        // This is a little gross but it creates the pool sizes required for the
        // DescriptorPoolCreateInfo passed into create_descriptor_pool. Do it here once instead of
        // in the allocator callback
        //
        let mut descriptor_counts = vec![0; dsc::DescriptorType::count()];
        for desc in &descriptor_set_layout_def.descriptor_set_layout_bindings {
            let ty: vk::DescriptorType = desc.descriptor_type.into();
            descriptor_counts[ty.as_raw() as usize] +=
                Self::MAX_DESCRIPTORS_PER_POOL * (1 + Self::MAX_FRAMES_IN_FLIGHT as u32);
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
            Self::MAX_FRAMES_IN_FLIGHT as u32,
            Self::MAX_FRAMES_IN_FLIGHT as u32 + 1,
            move |device| {
                let pool_builder = vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(Self::MAX_DESCRIPTORS_PER_POOL * (Self::MAX_FRAMES_IN_FLIGHT as u32 + 1))
                    .pool_sizes(&pool_sizes);

                unsafe { device.create_descriptor_pool(&*pool_builder, None) }
            },
        );

        let mut buffer_infos = Vec::new();
        for binding in &descriptor_set_layout_def.descriptor_set_layout_bindings {
            if let Some(per_descriptor_size) = binding.internal_buffer_per_descriptor_size {
                //TODO: 256 is the max allowed by the vulkan spec but we could improve this by using the
                // actual hardware value given by device limits
                let required_alignment = device_context.limits().min_uniform_buffer_offset_alignment as u32;
                let per_descriptor_stride = renderer_shell_vulkan::util::round_size_up_to_alignment_u32(per_descriptor_size, required_alignment);

                buffer_infos.push(DescriptorSetPoolRequiredBufferInfo {
                    per_descriptor_size,
                    per_descriptor_stride,
                    descriptor_type: binding.descriptor_type,
                    dst_element: DescriptorSetElementKey {
                        dst_binding: binding.binding
                    }
                })
            }
        }

        RegisteredDescriptorSetPool {
            slab: RawSlab::with_capacity(Self::MAX_DESCRIPTORS_PER_POOL),
            drop_tx,
            drop_rx,
            write_set_tx,
            write_set_rx,
            write_buffer_tx,
            write_buffer_rx,
            descriptor_pool_allocator,
            descriptor_set_layout,
            chunks: Default::default(),
            buffer_infos,
            buffer_drop_sink: VkResourceDropSink::new(Self::MAX_FRAMES_IN_FLIGHT as u32)
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
        let chunk_index = (slab_key.index() / Self::MAX_DESCRIPTORS_PER_POOL) as usize;

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
            self.chunks[chunk_index].write_set(slab_key, write_set, frame_in_flight_index);

        // Return the ref-counted descriptor set
        let descriptor_set =
            DescriptorSetArc::new(slab_key, descriptor_sets_per_frame, self.drop_tx.clone());
        Ok(descriptor_set)
    }

    //TODO: May need to decouple flushing writes from frame changes
    pub fn update(
        &mut self,
        device_context: &VkDeviceContext,
        frame_in_flight_index: FrameInFlightIndex,
    ) {
        // Route messages that indicate a dropped descriptor set to the chunk that owns it
        for dropped in self.drop_rx.try_iter() {
            self.slab.free(dropped);
        }

        for write in self.write_set_rx.try_iter() {
            let chunk_index = write.slab_key.index() / Self::MAX_DESCRIPTORS_PER_POOL;
            self.chunks[chunk_index as usize].write_set(write.slab_key, write.write_set, frame_in_flight_index);
        }

        for write in self.write_buffer_rx.try_iter() {
            let chunk_index = write.slab_key.index() / Self::MAX_DESCRIPTORS_PER_POOL;
            self.chunks[chunk_index as usize].write_buffer(write.slab_key, write.write_buffer, frame_in_flight_index);
        }

        // Commit pending writes/removes, rotate to the descriptor set for the next frame
        for chunk in &mut self.chunks {
            chunk.update(
                device_context,
                frame_in_flight_index,
            );
        }

        self.descriptor_pool_allocator
            .update(device_context.device());
    }

    // pub fn flush_changes(
    //     &mut self,
    //     device_context: &VkDeviceContext,
    //     frame_in_flight_index: FrameInFlightIndex,
    // ) {
    //
    // }

    pub fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        for chunk in &mut self.chunks {
            chunk.destroy(&mut self.descriptor_pool_allocator, &mut self.buffer_drop_sink);
        }

        self.descriptor_pool_allocator
            .destroy(device_context.device());
        self.buffer_drop_sink.destroy(&device_context);
        self.chunks.clear();
    }
}

#[derive(Debug)]
pub struct RegisteredDescriptorSetPoolMetrics {
    pub hash: ResourceHash,
    pub allocated_count: usize,
}

#[derive(Debug)]
pub struct RegisteredDescriptorSetPoolManagerMetrics {
    pub pools: Vec<RegisteredDescriptorSetPoolMetrics>,
}

pub struct RegisteredDescriptorSetPoolManager {
    device_context: VkDeviceContext,
    pools: FnvHashMap<ResourceHash, RegisteredDescriptorSetPool>,
    frame_in_flight_index: FrameInFlightIndex,
}

impl RegisteredDescriptorSetPoolManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        RegisteredDescriptorSetPoolManager {
            device_context: device_context.clone(),
            pools: Default::default(),
            frame_in_flight_index: 0,
        }
    }

    pub fn metrics(&self) -> RegisteredDescriptorSetPoolManagerMetrics {
        let mut registered_descriptor_sets_stats = Vec::with_capacity(self.pools.len());
        for (hash, value) in &self.pools {
            let pool_stats = RegisteredDescriptorSetPoolMetrics {
                hash: *hash,
                allocated_count: value.slab.allocated_count(),
            };
            registered_descriptor_sets_stats.push(pool_stats);
        }

        RegisteredDescriptorSetPoolManagerMetrics {
            pools: registered_descriptor_sets_stats,
        }
    }

    pub fn descriptor_set_for_current_frame(
        &self,
        descriptor_set_arc: &DescriptorSetArc,
    ) -> vk::DescriptorSet {
        descriptor_set_arc.inner.descriptor_sets_per_frame[self.frame_in_flight_index as usize]
    }

    pub fn insert(
        &mut self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
        write_set: DescriptorSetWriteSet,
    ) -> VkResult<DescriptorSetArc> {
        let hash = ResourceHash::from_key(descriptor_set_layout_def);
        let device_context = self.device_context.clone();
        let pool = self.pools.entry(hash).or_insert_with(|| {
            RegisteredDescriptorSetPool::new(
                &device_context,
                descriptor_set_layout_def,
                descriptor_set_layout,
            )
        });

        pool.insert(&self.device_context, write_set, self.frame_in_flight_index)
    }

    //TODO: Is creating and immediately modifying causing multiple writes?
    fn do_create_dyn_descriptor_set(
        &mut self,
        write_set: DescriptorSetWriteSet,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> VkResult<DynDescriptorSet> {
        // Get or create the pool for the layout
        let hash = ResourceHash::from_key(descriptor_set_layout_def);
        let device_context = self.device_context.clone();
        let pool = self.pools.entry(hash).or_insert_with(|| {
            RegisteredDescriptorSetPool::new(
                &device_context,
                descriptor_set_layout_def,
                descriptor_set_layout,
            )
        });

        // Allocate a descriptor set
        let descriptor_set = pool.insert(&self.device_context, write_set.clone(), self.frame_in_flight_index)?;

        // Create the DynDescriptorSet
        let dyn_descriptor_set = DynDescriptorSet::new(
            write_set,
            descriptor_set,
            pool.write_set_tx.clone(),
            pool.write_buffer_tx.clone(),
        );

        Ok(dyn_descriptor_set)
    }

    pub fn create_dyn_descriptor_set_uninitialized(
        &mut self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<DescriptorSetLayoutResource>,
    ) -> VkResult<DynDescriptorSet> {
        let write_set = create_uninitialized_write_set_for_layout(descriptor_set_layout_def);
        self.do_create_dyn_descriptor_set(write_set, descriptor_set_layout_def, descriptor_set_layout)
    }

    pub fn create_dyn_pass_material_instance_uninitialized(
        &mut self,
        pass: &LoadedMaterialPass,
        loaded_assets: &LoadedAssetLookupSet,
    ) -> VkResult<DynPassMaterialInstance> {
        let mut dyn_descriptor_sets = Vec::with_capacity(pass.descriptor_set_layouts.len());

        let layout_defs = &pass.pipeline_create_data.pipeline_layout_def.descriptor_set_layouts;
        for (layout_def, layout) in layout_defs.iter().zip(&pass.descriptor_set_layouts) {
            let dyn_descriptor_set = self.create_dyn_descriptor_set_uninitialized(layout_def, layout.clone())?;
            dyn_descriptor_sets.push(dyn_descriptor_set);
        }

        let dyn_pass_material_instance = DynPassMaterialInstance {
            descriptor_sets: dyn_descriptor_sets,
            slot_name_lookup: pass.pass_slot_name_lookup.clone()
        };

        Ok(dyn_pass_material_instance)
    }

    pub fn create_dyn_pass_material_instance_from_asset(
        &mut self,
        pass: &LoadedMaterialPass,
        material_instance: &LoadedMaterialInstance,
        loaded_assets: &LoadedAssetLookupSet,
        resources: &mut ResourceLookupSet,
    ) -> VkResult<DynPassMaterialInstance> {
        let write_sets = create_write_sets_for_material_instance_pass(
            pass,
            &material_instance.slot_assignments,
            loaded_assets,
            resources
        )?;

        let mut dyn_descriptor_sets = Vec::with_capacity(write_sets.len());

        for (layout_index, write_set) in write_sets.into_iter().enumerate() {
            let layout = &pass.descriptor_set_layouts[layout_index];
            let layout_def = &pass.pipeline_create_data.pipeline_layout_def.descriptor_set_layouts[layout_index];

            let dyn_descriptor_set = self.do_create_dyn_descriptor_set(write_set, layout_def, layout.clone())?;
            dyn_descriptor_sets.push(dyn_descriptor_set);
        }

        let dyn_pass_material_instance = DynPassMaterialInstance {
            descriptor_sets: dyn_descriptor_sets,
            slot_name_lookup: pass.pass_slot_name_lookup.clone()
        };

        Ok(dyn_pass_material_instance)
    }

    pub fn create_dyn_material_instance_uninitialized(
        &mut self,
        material: &LoadedMaterial,
        loaded_assets: &LoadedAssetLookupSet,
    ) -> VkResult<DynMaterialInstance> {
        let mut passes = Vec::with_capacity(material.passes.len());
        for pass in &material.passes {
            let dyn_pass_material_instance = self.create_dyn_pass_material_instance_uninitialized(pass, loaded_assets)?;
            passes.push(dyn_pass_material_instance);
        }

        Ok(DynMaterialInstance {
            passes
        })
    }

    pub fn create_dyn_material_instance_from_asset(
        &mut self,
        material: &LoadedMaterial,
        material_instance: &LoadedMaterialInstance,
        loaded_assets: &LoadedAssetLookupSet,
        resources: &mut ResourceLookupSet
    ) -> VkResult<DynMaterialInstance> {
        let mut passes = Vec::with_capacity(material.passes.len());
        for pass in &material.passes {
            let dyn_pass_material_instance = self.create_dyn_pass_material_instance_from_asset(
                pass,
                material_instance,
                loaded_assets,
                resources
            )?;
            passes.push(dyn_pass_material_instance);
        }

        Ok(DynMaterialInstance {
            passes
        })
    }

    pub fn update(&mut self) {
        self.frame_in_flight_index += 1;
        if self.frame_in_flight_index
            >= RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT as u32 + 1
        {
            self.frame_in_flight_index = 0;
        }

        for pool in self.pools.values_mut() {
            pool.update(&self.device_context, self.frame_in_flight_index);
        }
    }

    // pub fn flush_changes(&mut self) {
    //     for pool in self.pools.values_mut() {
    //         pool.flush_changes(&self.device_context, self.frame_in_flight_index);
    //     }
    // }

    pub fn destroy(&mut self) {
        for (hash, pool) in &mut self.pools {
            pool.destroy(&self.device_context);
        }

        self.pools.clear();
    }
}

#[derive(Default, Debug)]
pub struct WhatToBind {
    bind_samplers: bool,
    bind_images: bool,
    bind_buffers: bool,
}

pub fn what_to_bind(
    element_write: &DescriptorSetElementWrite,
) -> WhatToBind {
    let mut what = WhatToBind::default();

    // See https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkWriteDescriptorSet.html
    match element_write.descriptor_type {
        dsc::DescriptorType::Sampler => {
            what.bind_samplers = !element_write.has_immutable_sampler;
        }
        dsc::DescriptorType::CombinedImageSampler => {
            what.bind_samplers = !element_write.has_immutable_sampler;
            what.bind_images = true;
        },
        dsc::DescriptorType::SampledImage => {
            what.bind_images = true;
        }
        dsc::DescriptorType::UniformBuffer => {
            what.bind_buffers = true;
        }
        _ => unimplemented!(),
    }

    what
}

pub fn create_uninitialized_write_set_for_layout(layout: &dsc::DescriptorSetLayout) -> DescriptorSetWriteSet {
    let mut write_set = DescriptorSetWriteSet::default();
    for (binding_index, binding) in
        layout.descriptor_set_layout_bindings.iter().enumerate()
    {
        let key = DescriptorSetElementKey {
            dst_binding: binding_index as u32,
            //dst_array_element: 0,
        };

        let mut element_write = DescriptorSetElementWrite {
            has_immutable_sampler: binding.immutable_samplers.is_some(),
            descriptor_type: binding.descriptor_type.into(),
            image_info: Default::default(),
            buffer_info: Default::default(),
        };

        let what_to_bind = what_to_bind(&element_write);

        if what_to_bind.bind_images || what_to_bind.bind_samplers {
            element_write.image_info.resize(binding.descriptor_count as usize, DescriptorSetWriteElementImage::default());
        }

        if what_to_bind.bind_buffers {
            element_write.buffer_info.resize(binding.descriptor_count as usize, DescriptorSetWriteElementBuffer::default());
        }

        write_set.elements.insert(key, element_write);
    }

    write_set
}


pub fn apply_material_instance_slot_assignment(
    slot_assignment: &MaterialInstanceSlotAssignment,
    pass_slot_name_lookup: &SlotNameLookup,
    assets: &LoadedAssetLookupSet,
    resources: &mut ResourceLookupSet,
    material_pass_write_set: &mut Vec<DescriptorSetWriteSet>
) -> VkResult<()> {
    if let Some(slot_locations) = pass_slot_name_lookup.get(&slot_assignment.slot_name) {
        for location in slot_locations {
            let mut layout_descriptor_set_writes = &mut material_pass_write_set[location.layout_index as usize];
            let write = layout_descriptor_set_writes.elements.get_mut(&DescriptorSetElementKey {
                dst_binding: location.binding_index,
                //dst_array_element: location.array_index
            }).unwrap();

            let what_to_bind = what_to_bind(write);

            if what_to_bind.bind_images || what_to_bind.bind_samplers {
                let mut write_image = DescriptorSetWriteElementImage {
                    image_view: None,
                    sampler: None,
                };

                if what_to_bind.bind_images {
                    if let Some(image) = &slot_assignment.image {
                        let loaded_image = assets
                            .images
                            .get_latest(image.load_handle())
                            .unwrap();
                        write_image.image_view = Some(loaded_image.image_view.clone());
                    }
                }

                if what_to_bind.bind_samplers {
                    if let Some(sampler) = &slot_assignment.sampler {
                        let sampler = resources.get_or_create_sampler(sampler)?;
                        write_image.sampler = Some(sampler);
                    }
                }

                write.image_info = vec![write_image];
            }
        }
    }

    Ok(())
}

pub fn create_uninitialized_write_sets_for_material_pass(
    pass: &LoadedMaterialPass,
) -> Vec<DescriptorSetWriteSet> {
    // The metadata for the descriptor sets within this pass, one for each set within the pass
    let descriptor_set_layouts = &pass.shader_interface.descriptor_set_layouts;

    let mut pass_descriptor_set_writes : Vec<_> = descriptor_set_layouts.iter()
        .map(|layout| create_uninitialized_write_set_for_layout(&layout.into()))
        .collect();

    pass_descriptor_set_writes
}

pub fn create_write_sets_for_material_instance_pass(
    pass: &LoadedMaterialPass,
    slots: &Vec<MaterialInstanceSlotAssignment>,
    assets: &LoadedAssetLookupSet,
    resources: &mut ResourceLookupSet,
) -> VkResult<Vec<DescriptorSetWriteSet>> {
    let mut pass_descriptor_set_writes = create_uninitialized_write_sets_for_material_pass(pass);

    //
    // Now modify the descriptor set writes to actually point at the things specified by the material
    //
    for slot in slots {
        apply_material_instance_slot_assignment(
            slot,
            &pass.pass_slot_name_lookup,
            assets,
            resources,
            &mut pass_descriptor_set_writes
        )?;
    }

    Ok(pass_descriptor_set_writes)
}

pub struct DynDescriptorSet {
    descriptor_set: DescriptorSetArc,
    write_set: DescriptorSetWriteSet,

    write_set_tx: Sender<SlabKeyDescriptorSetWriteSet>,
    write_buffer_tx: Sender<SlabKeyDescriptorSetWriteBuffer>,

    //dirty: FnvHashSet<DescriptorSetElementKey>,

    pending_write_set: DescriptorSetWriteSet,
    pending_write_buffer: DescriptorSetWriteBuffer,
}

impl DynDescriptorSet {
    fn new(
        write_set: DescriptorSetWriteSet,
        descriptor_set: DescriptorSetArc,
        write_set_tx: Sender<SlabKeyDescriptorSetWriteSet>,
        write_buffer_tx: Sender<SlabKeyDescriptorSetWriteBuffer>,
    ) -> Self {
        DynDescriptorSet {
            descriptor_set,
            write_set,
            write_set_tx,
            write_buffer_tx,
            //dirty: Default::default(),
            pending_write_set: Default::default(),
            pending_write_buffer: Default::default(),
        }
    }

    pub fn descriptor_set(&self) -> &DescriptorSetArc {
        &self.descriptor_set
    }

    //TODO: Make a commit-like API so that it's not so easy to forget to call flush
    pub fn flush(&mut self) {
        if !self.pending_write_set.elements.is_empty() {
            let mut pending_write_set = Default::default();
            std::mem::swap(&mut pending_write_set, &mut self.pending_write_set);

            let pending_descriptor_set_write = SlabKeyDescriptorSetWriteSet {
                write_set: pending_write_set,
                slab_key: self.descriptor_set.inner.slab_key,
            };

            self.write_set_tx.send(pending_descriptor_set_write);
        }

        if !self.pending_write_buffer.elements.is_empty() {
            let mut pending_write_buffer = Default::default();
            std::mem::swap(&mut pending_write_buffer, &mut self.pending_write_buffer);

            let pending_descriptor_set_write = SlabKeyDescriptorSetWriteBuffer {
                write_buffer: pending_write_buffer,
                slab_key: self.descriptor_set.inner.slab_key,
            };

            self.write_buffer_tx.send(pending_descriptor_set_write);
        }
    }

    pub fn set_image(
        &mut self,
        binding_index: u32,
        image_view: ResourceArc<ImageViewResource>
    ) {
        self.set_image_array_element(binding_index, 0, image_view)
    }

    pub fn set_image_array_element(
        &mut self,
        binding_index: u32,
        array_index: usize,
        image_view: ResourceArc<ImageViewResource>
    ) {
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            //dst_array_element: 0
        };

        if let Some(element) = self.write_set.elements.get_mut(&key) {
            let what_to_bind = what_to_bind(element);
            if what_to_bind.bind_images {
                if let Some(element_image) = element.image_info.get_mut(array_index) {
                    element_image.image_view = Some(image_view);

                    self.pending_write_set.elements.insert(key, element.clone());

                    //self.dirty.insert(key);
                } else {
                    log::warn!("Tried to set image index {} but it did not exist. The image array is {} elements long.", array_index, element.image_info.len());
                }
            } else {
                // This is not necessarily an error if the user is binding with a slot name (although not sure
                // if that's the right approach long term)
                //log::warn!("Tried to bind an image to a descriptor set where the type does not accept an image", array_index)
            }
        } else {
            log::warn!("Tried to set image on a binding index that does not exist");
        }
    }

    pub fn set_buffer_data<T: Copy>(
        &mut self,
        binding_index: u32,
        data: &T
    ) {
        self.set_buffer_data_array_element(binding_index, 0, data)
    }

    fn set_buffer_data_array_element<T: Copy>(
        &mut self,
        binding_index: u32,
        array_index: usize,
        data: &T
    ) {
        //TODO: Verify that T's size matches the buffer

        // Not supporting array indices yet
        assert!(array_index == 0);
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            //dst_array_element: 0
        };

        if let Some(element) = self.write_set.elements.get_mut(&key) {
            let what_to_bind = what_to_bind(element);
            if what_to_bind.bind_buffers {
                let data = renderer_shell_vulkan::util::any_as_bytes(data).into();
                if element.buffer_info.len() > array_index {
                    self.pending_write_buffer.elements.insert(key, data);
                } else {
                    log::warn!("Tried to set buffer data for index {} but it did not exist. The buffer array is {} elements long.", array_index, element.buffer_info.len());
                }
            } else {
                // This is not necessarily an error if the user is binding with a slot name (although not sure
                // if that's the right approach long term)
                //log::warn!("Tried to bind an image to a descriptor set where the type does not accept an image", array_index)
            }
        } else {
            log::warn!("Tried to set buffer data on a binding index that does not exist");
        }
    }
}

pub struct DynPassMaterialInstance {
    descriptor_sets: Vec<DynDescriptorSet>,
    slot_name_lookup: Arc<SlotNameLookup>,
}

impl DynPassMaterialInstance {
    pub fn descriptor_set_layout(&self, layout_index: u32) -> &DynDescriptorSet {
        &self.descriptor_sets[layout_index as usize]
    }

    pub fn flush(&mut self) {
        for set in &mut self.descriptor_sets {
            set.flush()
        }
    }

    pub fn set_image(
        &mut self,
        slot_name: &String,
        image_view: ResourceArc<ImageViewResource>
    ) {
        if let Some(slot_locations) = self.slot_name_lookup.get(slot_name) {
            for slot_location in slot_locations {
                if let Some(dyn_descriptor_set) = self.descriptor_sets.get_mut(slot_location.layout_index as usize) {
                    dyn_descriptor_set.set_image(slot_location.binding_index, image_view.clone());
                }
            }
        }
    }

    pub fn set_buffer_data<T: Copy>(
        &mut self,
        slot_name: &String,
        data: &T
    ) {
        if let Some(slot_locations) = self.slot_name_lookup.get(slot_name) {
            for slot_location in slot_locations {
                if let Some(dyn_descriptor_set) = self.descriptor_sets.get_mut(slot_location.layout_index as usize) {
                    dyn_descriptor_set.set_buffer_data(slot_location.binding_index, data);
                }
            }
        }
    }
}

pub struct DynMaterialInstance {
    passes: Vec<DynPassMaterialInstance>,
}

impl DynMaterialInstance {
    pub fn pass(&self, pass_index: u32) -> &DynPassMaterialInstance {
        &self.passes[pass_index as usize]
    }

    pub fn flush(&mut self) {
        for pass in &mut self.passes {
            pass.flush()
        }
    }

    pub fn set_image(
        &mut self,
        slot_name: &String,
        image_view: &ResourceArc<ImageViewResource>
    ) {
        for pass in &mut self.passes {
            pass.set_image(slot_name, image_view.clone())
        }
    }

    pub fn set_buffer_data<T: Copy>(
        &mut self,
        slot_name: &String,
        data: &T
    ) {
        for pass in &mut self.passes {
            pass.set_buffer_data(slot_name, data)
        }
    }
}
