use super::resource_lookup::ResourceArc;
use ash::vk;
use renderer_base::slab::{RawSlabKey, RawSlab};
use crossbeam_channel::{Sender, Receiver};
use std::fmt::Formatter;
use std::sync::Arc;
use std::collections::VecDeque;
use renderer_shell_vulkan::{VkDeviceContext, VkDescriptorPoolAllocator};
use ash::prelude::VkResult;
use fnv::FnvHashMap;
use super::ResourceHash;
use crate::pipeline_description as dsc;
use ash::version::DeviceV1_0;

//
// These represent a write update that can be applied to a descriptor set in a pool
//
#[derive(Debug)]
pub struct DescriptorSetWriteImage {
    pub sampler: Option<ResourceArc<vk::Sampler>>,
    pub image_view: Option<ResourceArc<vk::ImageView>>,
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
//     pub fn set_image_view(&mut self, image_view: ResourceArc<vk::ImageView>) {
//         self.image_info.image_view = image_view.get_raw();
//         self.image_view = Some(image_view);
//     }
// }

#[derive(Debug)]
pub struct DescriptorSetWriteBuffer {
    pub buffer: ResourceArc<vk::Buffer>,
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

#[derive(Debug)]
pub struct DescriptorSetWrite {
    //pub dst_set: u32, // a pool index?
    //pub dst_layout: u32, // a hash?
    //pub dst_pool_index: u32, // a slab key?
    //pub dst_set_index: u32,

    //pub descriptor_set: DescriptorSetArc,
    pub dst_binding: u32,
    pub dst_array_element: u32,
    pub descriptor_type: dsc::DescriptorType,
    pub image_info: Vec<DescriptorSetWriteImage>,
    pub buffer_info: Vec<DescriptorSetWriteBuffer>,
    //pub p_texel_buffer_view: *const BufferView,
}

struct DescriptorWriteBuilder {
    image_infos: Vec<vk::DescriptorImageInfo>,
    buffer_infos: Vec<vk::DescriptorBufferInfo>,

}

// impl DescriptorSetWrite {
//     fn write_sets(
//         desciptor_set: vk::DescriptorSet,
//         writes: &[&DescriptorSetWrite]
//     ) {
//         // This function is a bit tricky unfortunately. We need to build a list of vk::WriteDescriptorSet
//         // but this struct has a pointer to data in image_infos/buffer_infos. To deal with this, we
//         // need to push the temporary lists of these infos into these lists. This way they don't
//         // drop out of scope while we are using them. Ash does do some lifetime tracking, but once
//         // you call build() it completely trusts that any pointers it holds will stay valid. So
//         // while these lists are mutable to allow pushing data in, the Vecs inside must not be modified.
//         let mut vk_image_infos = Vec::with_capacity(writes.len());
//         //let mut vk_buffer_infos = Vec::with_capacity(writes.len());
//
//         for write in writes {
//             let mut builder = vk::WriteDescriptorSet::builder()
//                 .dst_set(desciptor_set)
//                 .dst_binding(write.dst_binding)
//                 .dst_array_element(write.dst_array_element)
//                 .descriptor_type(write.descriptor_type.into());
//
//             if !write.image_info.is_empty() {
//                 let mut image_infos = &write.image_info;
//                 for image_info in image_infos {
//                     let mut image_info_builder = vk::DescriptorImageInfo::builder();
//                     image_info_builder = image_info_builder.image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
//                     if let Some(image_view) = &image_info.image_view {
//                         image_info_builder = image_info_builder.image_view(image_view.get_raw());
//                     }
//                     if let Some(sampler) = &image_info.sampler {
//                         image_info_builder = image_info_builder.sampler(sampler.get_raw());
//                     }
//
//                     vk_image_infos.push(image_info_builder.build());
//                 }
//
//                 builder = builder.image_info(&vk_image_infos);
//             }
//
//             if !write.buffer_info.is_empty() {
//             //if let Some(buffer_infos) = &write.buffer_info {
//                 let mut buffer_infos = &write.buffer_info;
//                 for buffer_info in buffer_infos {
//                     // Need to support buffers and knowing the size of them. Probably need to use
//                     // ResourceArc<BufferRaw>
//                     unimplemented!();
//                     // let mut buffer_info_builder = vk::DescriptorBufferInfo::builder()
//                     //     .buffer(buffer_info.buffer)
//                     //     .offset(0)
//                     //     .range()
//                 }
//
//                 builder = builder.buffer_info(&vk_buffer_infos);
//             }
//
//             //builder = builder.texel_buffer_view();
//         }
//
//
//
//
//     }
// }

struct RegisteredDescriptorSet {
    // Anything we'd want to store per descriptor set can go here, but don't have anything yet
}


type FrameInFlightIndex = u32;

//
// Reference counting mechanism to keep descriptor sets allocated
//
struct DescriptorSetArcInner {
    // We can't cache the vk::DescriptorSet here because the pools will be cycled
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    descriptor_sets_per_frame: Vec<vk::DescriptorSet>,
    drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>
}

impl std::fmt::Debug for DescriptorSetArcInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DescriptorSetArcInner")
            .field("slab_key", &self.slab_key)
            .finish()
    }
}

pub struct DescriptorSetArc {
    inner: Arc<DescriptorSetArcInner>
}

impl DescriptorSetArc {
    fn new(
        slab_key: RawSlabKey<RegisteredDescriptorSet>,
        descriptor_sets_per_frame: Vec<vk::DescriptorSet>,
        drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>
    ) -> Self {
        let inner = DescriptorSetArcInner {
            slab_key,
            descriptor_sets_per_frame,
            drop_tx,
        };

        DescriptorSetArc {
            inner: Arc::new(inner)
        }
    }
}

impl std::fmt::Debug for DescriptorSetArc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DescriptorSetArc")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Debug)]
struct PendingDescriptorSetWrite {
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    writes: Vec<DescriptorSetWrite>,
    live_until_frame: FrameInFlightIndex,
}

// struct PendingDescriptorSetRemove {
//     writes: Vec<DescriptorSetWrite>,
//     live_until_frame: Wrapping<u32>,
// }

struct RegisteredDescriptorSetPoolChunk {
    // One per frame
    //pools: Vec<vk::DescriptorPool>,
    pool: vk::DescriptorPool,
    descriptor_sets: Vec<Vec<vk::DescriptorSet>>,

    // These are stored for RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT frames so that they
    // are applied to each frame's pool
    pending_writes: VecDeque<PendingDescriptorSetWrite>,

    //EDIT: This is probably unnecessary
    // // These are stored for RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT so that the index
    // // is not free until all frame flush
    // pending_removes: Vec<PendingDescriptorSetWrite>,
}

impl RegisteredDescriptorSetPoolChunk {
    fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout: vk::DescriptorSetLayout,
        allocator: &mut VkDescriptorPoolAllocator
    ) -> VkResult<Self> {

        let pool = allocator.allocate_pool(device_context.device())?;

        let descriptor_set_layouts = [descriptor_set_layout; RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1];

        let mut descriptor_sets = Vec::with_capacity(RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1);
        for i in 0..RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1 {
            let set_create_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&descriptor_set_layouts);


            let descriptor_sets_for_frame = unsafe {
                device_context.device().allocate_descriptor_sets(&*set_create_info)?
            };
            descriptor_sets.push(descriptor_sets_for_frame);
        }

        Ok(RegisteredDescriptorSetPoolChunk {
            pool,
            descriptor_sets,
            pending_writes: Default::default()
        })
    }

    pub fn destroy(&mut self, allocator: &mut VkDescriptorPoolAllocator) {
        // for pool in &mut self.pools {
        //     allocator.retire_pool(*pool);
        // }
        allocator.retire_pool(self.pool);
        //self.pools.clear();
    }

    fn write(
        &mut self,
        slab_key: RawSlabKey<RegisteredDescriptorSet>,
        mut writes: Vec<DescriptorSetWrite>,
        frame_in_flight_index: FrameInFlightIndex,
    ) -> Vec<vk::DescriptorSet> {
        log::debug!("Schedule a write for descriptor set {:?}\n{:#?}", slab_key, writes);
        // Use frame_in_flight_index for the live_until_frame because every update, we immediately
        // increment the frame and *then* do updates. So by setting it to the pre-next-update
        // frame_in_flight_index, this will make the write stick around for MAX_FRAMES_IN_FLIGHT frames
        let pending_write = PendingDescriptorSetWrite {
            slab_key,
            writes: writes,
            live_until_frame: frame_in_flight_index,
        };

        //TODO: Queue writes to occur for next N frames
        self.pending_writes.push_back(pending_write);

        let descriptor_index = slab_key.index() % RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL;
        self.descriptor_sets.iter().map(|x| x[descriptor_index as usize]).collect()
    }

    // fn remove(&mut self, slab_key: RawSlabKey<RegisteredDescriptorSet>) {
    //     let pending_write = PendingDescriptorSetRemove {
    //         slab_key,
    //         live_until_frame: frame_in_flight_index + RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1,
    //     };
    //
    //     self.writes.append(&mut writes);
    // }

    fn update(
        &mut self,
        device_context: &VkDeviceContext,
        //slab: &mut RawSlab<RegisteredDescriptorSet>,
        frame_in_flight_index: FrameInFlightIndex
    ) {
        // This function is a bit tricky unfortunately. We need to build a list of vk::WriteDescriptorSet
        // but this struct has a pointer to data in image_infos/buffer_infos. To deal with this, we
        // need to push the temporary lists of these infos into these lists. This way they don't
        // drop out of scope while we are using them. Ash does do some lifetime tracking, but once
        // you call build() it completely trusts that any pointers it holds will stay valid. So
        // while these lists are mutable to allow pushing data in, the Vecs inside must not be modified.
        let mut vk_image_infos = vec![];
        //let mut vk_buffer_infos = vec![];

        let mut write_builders = vec![];
        for pending_write in &self.pending_writes {
            log::debug!("Process descriptor set pending_write for {:?} frame {}\n{:#?}", pending_write.slab_key, frame_in_flight_index, pending_write);
            for write in &pending_write.writes {
                //writes.push(write);

                let descriptor_set_index = pending_write.slab_key.index() % RegisteredDescriptorSetPool::MAX_DESCRIPTORS_PER_POOL;
                let descriptor_set = self.descriptor_sets[frame_in_flight_index as usize][descriptor_set_index as usize];

                let mut builder = vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(write.dst_binding)
                    .dst_array_element(write.dst_array_element)
                    .descriptor_type(write.descriptor_type.into());

                let mut image_infos = Vec::with_capacity(write.image_info.len());
                if !write.image_info.is_empty() {
                    for image_info in &write.image_info {
                        let mut image_info_builder = vk::DescriptorImageInfo::builder();
                        image_info_builder = image_info_builder.image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                        if let Some(image_view) = &image_info.image_view {
                            image_info_builder = image_info_builder.image_view(image_view.get_raw());
                        }
                        if let Some(sampler) = &image_info.sampler {
                            image_info_builder = image_info_builder.sampler(sampler.get_raw());
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
        }

        //DescriptorSetWrite::write_sets(self.sets[frame_in_flight_index], writes);

        //device_context.device().update_descriptor_sets()

        if !write_builders.is_empty() {
            unsafe {
                device_context.device().update_descriptor_sets(&write_builders, &[]);
            }
        }


        // Determine how many writes we can drain
        let mut pending_writes_to_drain = 0;
        for pending_write in &self.pending_writes {
            // If frame_in_flight_index matches or exceeds live_until_frame, then the result will be a very
            // high value due to wrapping a negative value to u32::MAX
            if pending_write.live_until_frame == frame_in_flight_index {
                pending_writes_to_drain += 1;
            } else {
                break;
            }
        }

        // Drop any writes that have lived long enough to apply to the descriptor set for each frame
        self.pending_writes.drain(0..pending_writes_to_drain);
    }
}

struct RegisteredDescriptorSetPool {
    //descriptor_set_layout_def: dsc::DescriptorSetLayout,
    slab: RawSlab<RegisteredDescriptorSet>,
    //pending_allocations: Vec<DescriptorSetWrite>,
    drop_tx: Sender<RawSlabKey<RegisteredDescriptorSet>>,
    drop_rx: Receiver<RawSlabKey<RegisteredDescriptorSet>>,
    descriptor_pool_allocator: VkDescriptorPoolAllocator,
    descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,

    chunks: Vec<RegisteredDescriptorSetPoolChunk>,
}

impl RegisteredDescriptorSetPool {
    const MAX_DESCRIPTORS_PER_POOL : u32 = 64;
    const MAX_FRAMES_IN_FLIGHT : usize = renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT;

    pub fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
    ) -> Self {
        //renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        //
        // This is a little gross but it creates the pool sizes required for the
        // DescriptorPoolCreateInfo passed into create_descriptor_pool. Do it here once instead of
        // in the allocator callback
        //
        let mut descriptor_counts = vec![0; dsc::DescriptorType::count()];
        for desc in &descriptor_set_layout_def.descriptor_set_layout_bindings {
            let ty : vk::DescriptorType = desc.descriptor_type.into();
            descriptor_counts[ty.as_raw() as usize] += Self::MAX_DESCRIPTORS_PER_POOL * (1 + Self::MAX_FRAMES_IN_FLIGHT as u32);
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
                    .max_sets(Self::MAX_DESCRIPTORS_PER_POOL)
                    .pool_sizes(&pool_sizes);

                unsafe {
                    device.create_descriptor_pool(&*pool_builder, None)
                }
            }
        );

        RegisteredDescriptorSetPool {
            //descriptor_set_layout_def: descriptor_set_layout_def.clone(),
            slab: RawSlab::with_capacity(Self::MAX_DESCRIPTORS_PER_POOL),
            //pending_allocations: Default::default(),
            drop_tx,
            drop_rx,
            descriptor_pool_allocator,
            descriptor_set_layout,
            chunks: Default::default()
        }
    }

    pub fn insert(
        &mut self,
        device_context: &VkDeviceContext,
        writes: Vec<DescriptorSetWrite>,
        frame_in_flight_index: FrameInFlightIndex,
    ) -> VkResult<DescriptorSetArc> {
        let registered_set = RegisteredDescriptorSet {
            // Don't have anything to store yet
        };

        // Use the slab allocator to find an unused index, determine the chunk index from that
        let slab_key = self.slab.allocate(registered_set);
        let chunk_index = (slab_key.index() / Self::MAX_DESCRIPTORS_PER_POOL) as usize;

        // Add more chunks if necessary
        while chunk_index as usize >= self.chunks.len() {
            self.chunks.push(RegisteredDescriptorSetPoolChunk::new(
                device_context,
                self.descriptor_set_layout.get_raw(),
                &mut self.descriptor_pool_allocator
            )?);
        }

        // Insert the write into the chunk, it will be applied when update() is next called on it
        let descriptor_sets_per_frame = self.chunks[chunk_index].write(slab_key, writes, frame_in_flight_index);

        // Return the ref-counted descriptor set
        let descriptor_set = DescriptorSetArc::new(slab_key, descriptor_sets_per_frame, self.drop_tx.clone());
        Ok(descriptor_set)
    }

    pub fn update(&mut self, device_context: &VkDeviceContext, frame_in_flight_index: FrameInFlightIndex) {
        // Route messages that indicate a dropped descriptor set to the chunk that owns it
        for dropped in self.drop_rx.try_iter() {
            // let chunk_index = (dropped.index() / Self::MAX_DESCRIPTORS_PER_POOL) as usize;
            // self.chunks[chunk_index].remove(dropped);
            self.slab.free(dropped);
        }

        // Commit pending writes/removes, rotate to the descriptor set for the next frame
        for chunk in &mut self.chunks {
            chunk.update(
                device_context,
                //&mut self.slab,
                frame_in_flight_index
            );
        }

        self.descriptor_pool_allocator.update(device_context.device());
    }

    pub fn destroy(&mut self, device_context: &VkDeviceContext) {
        for chunk in &mut self.chunks {
            chunk.destroy(&mut self.descriptor_pool_allocator);
        }

        self.descriptor_pool_allocator.destroy(device_context.device());
        self.chunks.clear();
    }
}

#[derive(Debug)]
pub struct RegisteredDescriptorSetPoolStats {
    pub hash: ResourceHash,
    pub allocated_count: usize,
}

#[derive(Debug)]
pub struct RegisteredDescriptorSetPoolManagerStats {
    pub pools: Vec<RegisteredDescriptorSetPoolStats>
}

pub struct RegisteredDescriptorSetPoolManager {
    device_context: VkDeviceContext,
    pools: FnvHashMap<ResourceHash, RegisteredDescriptorSetPool>,
    frame_in_flight_index: FrameInFlightIndex,

}

impl RegisteredDescriptorSetPoolManager {
    pub fn new(
        device_context: &VkDeviceContext,
    ) -> Self {
        RegisteredDescriptorSetPoolManager {
            device_context: device_context.clone(),
            pools: Default::default(),
            frame_in_flight_index: 0
        }
    }

    pub fn metrics(&self) -> RegisteredDescriptorSetPoolManagerStats {
        let mut registered_descriptor_sets_stats = Vec::with_capacity(self.pools.len());
        for (hash, value) in &self.pools {
            let pool_stats = RegisteredDescriptorSetPoolStats {
                hash: *hash,
                allocated_count: value.slab.allocated_count()
            };
            registered_descriptor_sets_stats.push(pool_stats);
        }

        RegisteredDescriptorSetPoolManagerStats {
            pools: registered_descriptor_sets_stats
        }
    }

    pub fn descriptor_set(&self, descriptor_set_arc: &DescriptorSetArc) -> vk::DescriptorSet {
        descriptor_set_arc.inner.descriptor_sets_per_frame[self.frame_in_flight_index as usize]
    }

    pub fn insert(
        &mut self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
        //resources: &ResourceLookup<dsc::DescriptorSetLayout, vk::DescriptorSetLayout>,
        writes: Vec<DescriptorSetWrite>
    ) -> VkResult<DescriptorSetArc> {
        let hash = ResourceHash::from_key(descriptor_set_layout_def);

        let device_context = self.device_context.clone();
        let pool = self.pools.entry(hash)
            .or_insert_with(|| {
                RegisteredDescriptorSetPool::new(&device_context, descriptor_set_layout_def, descriptor_set_layout)
            });

        pool.insert(&device_context, writes, self.frame_in_flight_index)
    }

    pub fn update(&mut self) {
        self.frame_in_flight_index += 1;
        if self.frame_in_flight_index >= RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT as u32 + 1{
            self.frame_in_flight_index = 0;
        }

        for pool in self.pools.values_mut() {
            pool.update(&self.device_context, self.frame_in_flight_index);
        }
    }

    pub fn destroy(&mut self) {
        for (hash, pool) in &mut self.pools {
            pool.destroy(&self.device_context);
        }

        self.pools.clear();
    }
}






