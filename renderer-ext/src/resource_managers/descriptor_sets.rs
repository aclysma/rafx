use super::resource_lookup::ResourceArc;
use ash::vk;
use renderer_base::slab::{RawSlabKey, RawSlab};
use crossbeam_channel::{Sender, Receiver};
use std::fmt::Formatter;
use std::sync::Arc;
use std::collections::VecDeque;
use renderer_shell_vulkan::{VkDeviceContext, VkDescriptorPoolAllocator};
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

//
// These represent a write update that can be applied to a descriptor set in a pool
//
#[derive(Debug, Clone, Default)]
pub struct DescriptorSetWriteElementImage {
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

#[derive(Debug)]
struct SlabKeyDescriptorSetWriteSet {
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    write_set: DescriptorSetWriteSet,
}

struct DescriptorWriteBuilder {
    image_infos: Vec<vk::DescriptorImageInfo>,
    buffer_infos: Vec<vk::DescriptorBufferInfo>,
}

struct RegisteredDescriptorSet {
    // Anything we'd want to store per descriptor set can go here, but don't have anything yet
    write_set: DescriptorSetWriteSet,
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

#[derive(Debug)]
struct PendingDescriptorSetWrite {
    slab_key: RawSlabKey<RegisteredDescriptorSet>,
    write_set: DescriptorSetWriteSet,
    live_until_frame: FrameInFlightIndex,
}

struct RegisteredDescriptorSetPoolChunk {
    // One per frame
    //pools: Vec<vk::DescriptorPool>,
    pool: vk::DescriptorPool,
    descriptor_sets: Vec<Vec<vk::DescriptorSet>>,

    // These are stored for RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT frames so that they
    // are applied to each frame's pool
    pending_writes: VecDeque<PendingDescriptorSetWrite>,
}

impl RegisteredDescriptorSetPoolChunk {
    fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout: vk::DescriptorSetLayout,
        allocator: &mut VkDescriptorPoolAllocator,
    ) -> VkResult<Self> {
        let pool = allocator.allocate_pool(device_context.device())?;

        let descriptor_set_layouts =
            [descriptor_set_layout; RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1];

        let mut descriptor_sets =
            Vec::with_capacity(RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1);
        for i in 0..RegisteredDescriptorSetPool::MAX_FRAMES_IN_FLIGHT + 1 {
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

        Ok(RegisteredDescriptorSetPoolChunk {
            pool,
            descriptor_sets,
            pending_writes: Default::default(),
        })
    }

    fn destroy(
        &mut self,
        allocator: &mut VkDescriptorPoolAllocator,
    ) {
        allocator.retire_pool(self.pool);
    }

    fn write(
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
        let pending_write = PendingDescriptorSetWrite {
            slab_key,
            write_set,
            live_until_frame: frame_in_flight_index,
        };

        //TODO: Consider pushing these into a hashmap for the frame and let the pending write array
        // be a list of hashmaps
        self.pending_writes.push_back(pending_write);

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
        let mut all_writes = FnvHashMap::default();
        for pending_write in &self.pending_writes {
            for (key, value) in &pending_write.write_set.elements {
                all_writes.insert(SlabElementKey(pending_write.slab_key, *key), value);
            }
        }

        let mut write_builders = vec![];
        for (key, element) in all_writes {
            let slab_key = key.0;
            let element_key = key.1;

            log::trace!("Process descriptor set pending_write for {:?}", slab_key);
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
                    let mut image_info_builder = vk::DescriptorImageInfo::builder();
                    image_info_builder = image_info_builder
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                    if let Some(image_view) = &image_info.image_view {
                        image_info_builder =
                            image_info_builder.image_view(image_view.get_raw());
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

        //DescriptorSetWrite::write_sets(self.sets[frame_in_flight_index], writes);

        //device_context.device().update_descriptor_sets()

        if !write_builders.is_empty() {
            unsafe {
                device_context
                    .device()
                    .update_descriptor_sets(&write_builders, &[]);
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
    write_tx: Sender<SlabKeyDescriptorSetWriteSet>,
    write_rx: Receiver<SlabKeyDescriptorSetWriteSet>,
    descriptor_pool_allocator: VkDescriptorPoolAllocator,
    descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,

    chunks: Vec<RegisteredDescriptorSetPoolChunk>,
}

impl RegisteredDescriptorSetPool {
    const MAX_DESCRIPTORS_PER_POOL: u32 = 64;
    const MAX_FRAMES_IN_FLIGHT: usize = renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT;

    pub fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
    ) -> Self {
        //renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();
        let (write_tx, write_rx) = crossbeam_channel::unbounded();

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
                    .max_sets(Self::MAX_DESCRIPTORS_PER_POOL)
                    .pool_sizes(&pool_sizes);

                unsafe { device.create_descriptor_pool(&*pool_builder, None) }
            },
        );

        RegisteredDescriptorSetPool {
            //descriptor_set_layout_def: descriptor_set_layout_def.clone(),
            slab: RawSlab::with_capacity(Self::MAX_DESCRIPTORS_PER_POOL),
            //pending_allocations: Default::default(),
            drop_tx,
            drop_rx,
            write_tx,
            write_rx,
            descriptor_pool_allocator,
            descriptor_set_layout,
            chunks: Default::default(),
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
            write_set: write_set.clone()
        };

        // Use the slab allocator to find an unused index, determine the chunk index from that
        let slab_key = self.slab.allocate(registered_set);
        let chunk_index = (slab_key.index() / Self::MAX_DESCRIPTORS_PER_POOL) as usize;

        // Add more chunks if necessary
        while chunk_index as usize >= self.chunks.len() {
            self.chunks.push(RegisteredDescriptorSetPoolChunk::new(
                device_context,
                self.descriptor_set_layout.get_raw(),
                &mut self.descriptor_pool_allocator,
            )?);
        }

        // Insert the write into the chunk, it will be applied when update() is next called on it
        let descriptor_sets_per_frame =
            self.chunks[chunk_index].write(slab_key, write_set, frame_in_flight_index);

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

        for write in self.write_rx.try_iter() {
            let chunk_index = write.slab_key.index() / Self::MAX_DESCRIPTORS_PER_POOL;
            self.chunks[chunk_index as usize].write(write.slab_key, write.write_set, frame_in_flight_index);
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

    pub fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        for chunk in &mut self.chunks {
            chunk.destroy(&mut self.descriptor_pool_allocator);
        }

        self.descriptor_pool_allocator
            .destroy(device_context.device());
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
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
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
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
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
        let descriptor_set = pool.insert(&self.device_context, DescriptorSetWriteSet::default(), self.frame_in_flight_index)?;

        // Create the DynDescriptorSet
        let dyn_descriptor_set = DynDescriptorSet::new(write_set, descriptor_set, pool.write_tx.clone());

        Ok(dyn_descriptor_set)
    }

    pub fn create_dyn_descriptor_set_uninitialized(
        &mut self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
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
    ) -> VkResult<DynPassMaterialInstance> {
        let write_sets = create_write_sets_for_material_instance_pass(
            pass,
            &material_instance.slot_assignments,
            loaded_assets
        );

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
    ) -> VkResult<DynMaterialInstance> {
        let mut passes = Vec::with_capacity(material.passes.len());
        for pass in &material.passes {
            let dyn_pass_material_instance = self.create_dyn_pass_material_instance_from_asset(pass, material_instance, loaded_assets)?;
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

    pub fn destroy(&mut self) {
        for (hash, pool) in &mut self.pools {
            pool.destroy(&self.device_context);
        }

        self.pools.clear();
    }
}

#[derive(Default)]
pub struct WhatToBind {
    bind_samplers: bool,
    bind_images: bool,
    bind_buffers: bool,
}

pub fn what_to_bind(descriptor_type: dsc::DescriptorType) -> WhatToBind {
    let mut what = WhatToBind::default();

    // See https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkWriteDescriptorSet.html
    match descriptor_type {
        dsc::DescriptorType::Sampler => {
            //what.bind_samplers = true;
        }
        dsc::DescriptorType::CombinedImageSampler => {
            //what.bind_samplers = true;
            //what.bind_images = true;
        }
        dsc::DescriptorType::SampledImage => {
            what.bind_images = true;
        }
        dsc::DescriptorType::UniformBuffer => {
            //what.bind_buffers = true;
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
            descriptor_type: binding.descriptor_type.into(),
            image_info: Default::default(),
            buffer_info: Default::default(),
        };

        let what_to_bind = what_to_bind(binding.descriptor_type);

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
    material_pass_write_set: &mut Vec<DescriptorSetWriteSet>
) {
    if let Some(slot_locations) = pass_slot_name_lookup.get(&slot_assignment.slot_name) {
        for location in slot_locations {
            let mut layout_descriptor_set_writes = &mut material_pass_write_set[location.layout_index as usize];
            let write = layout_descriptor_set_writes.elements.get_mut(&DescriptorSetElementKey {
                dst_binding: location.binding_index,
                //dst_array_element: location.array_index
            }).unwrap();

            let mut bind_samplers = false;
            let mut bind_images = false;
            match write.descriptor_type {
                dsc::DescriptorType::Sampler => {
                    bind_samplers = true;
                }
                dsc::DescriptorType::CombinedImageSampler => {
                    bind_samplers = true;
                    bind_images = true;
                }
                dsc::DescriptorType::SampledImage => {
                    bind_images = true;
                }
                _ => unimplemented!(),
            }

            let mut write_image = DescriptorSetWriteElementImage {
                image_view: None,
                sampler: None,
            };

            if bind_images {
                if let Some(image) = &slot_assignment.image {
                    let loaded_image = assets
                        .images
                        .get_latest(image.load_handle())
                        .unwrap();
                    write_image.image_view = Some(loaded_image.image_view.clone());
                }

                write.image_info = vec![write_image];
            }
        }
    }
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
) -> Vec<DescriptorSetWriteSet> {
    let mut pass_descriptor_set_writes = create_uninitialized_write_sets_for_material_pass(pass);

    //
    // Now modify the descriptor set writes to actually point at the things specified by the material
    //
    for slot in slots {
        apply_material_instance_slot_assignment(slot, &pass.pass_slot_name_lookup, assets, &mut pass_descriptor_set_writes);
    }

    pass_descriptor_set_writes
}

pub struct DynDescriptorSet {
    descriptor_set: DescriptorSetArc,
    write_set: DescriptorSetWriteSet,
    write_tx: Sender<SlabKeyDescriptorSetWriteSet>,
    dirty: FnvHashSet<DescriptorSetElementKey>,
}

impl DynDescriptorSet {
    fn new(
        write_set: DescriptorSetWriteSet,
        descriptor_set: DescriptorSetArc,
        write_tx: Sender<SlabKeyDescriptorSetWriteSet>,
    ) -> Self {
        DynDescriptorSet {
            descriptor_set,
            write_set,
            write_tx,
            dirty: Default::default(),
        }
    }

    pub fn descriptor_set(&self) -> &DescriptorSetArc {
        &self.descriptor_set
    }

    //TODO: Make a commit-like API so that it's not so easy to forget to call flush
    pub fn flush(&mut self) {
        let mut write_set = DescriptorSetWriteSet::default();
        for dirty_element_key in self.dirty.drain() {
            let value = self.write_set.elements[&dirty_element_key].clone();
            write_set.elements.insert(dirty_element_key, value);
        }

        let pending_descriptor_set_write = SlabKeyDescriptorSetWriteSet {
            write_set,
            slab_key: self.descriptor_set.inner.slab_key,
        };

        self.write_tx.send(pending_descriptor_set_write);
    }

    pub fn set_image(
        &mut self,
        binding_index: u32,
        image_view: ResourceArc<vk::ImageView>
    ) {
        self.set_image_array_element(binding_index, 0, image_view)
    }

    pub fn set_image_array_element(
        &mut self,
        binding_index: u32,
        array_index: usize,
        image_view: ResourceArc<vk::ImageView>
    ) {
        let key = DescriptorSetElementKey {
            dst_binding: binding_index,
            //dst_array_element: 0
        };

        if let Some(x) = self.write_set.elements.get_mut(&key) {
            let what_to_bind = what_to_bind(x.descriptor_type);
            if what_to_bind.bind_images {
                if let Some(x) = x.image_info.get_mut(array_index) {
                    x.image_view = Some(image_view);
                    self.dirty.insert(key);
                } else {
                    log::warn!("Tried to set image index {} but it did not exist. The image array is {} elements long.", array_index, x.image_info.len());
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
        image_view: ResourceArc<vk::ImageView>
    ) {
        if let Some(slot_locations) = self.slot_name_lookup.get(slot_name) {
            for slot_location in slot_locations {
                if let Some(dyn_descriptor_set) = self.descriptor_sets.get_mut(slot_location.layout_index as usize) {
                    dyn_descriptor_set.set_image(slot_location.binding_index, image_view.clone());
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
        image_view: &ResourceArc<vk::ImageView>
    ) {
        for pass in &mut self.passes {
            pass.set_image(slot_name, image_view.clone())
        }
    }
}
