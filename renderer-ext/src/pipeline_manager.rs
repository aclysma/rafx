
use crate::pipeline_description as dsc;
use fnv::FnvHashMap;
use ash::vk;
use ash::version::DeviceV1_0;
use renderer_shell_vulkan::VkDeviceContext;
use ash::prelude::VkResult;
use std::collections::hash_map::Entry::Occupied;
use ash::vk::{PipelineDynamicStateCreateInfo};
use std::marker::PhantomData;
use atelier_assets::loader::LoadHandle;
use mopa::Any;
use std::ops::Deref;
use image::load;
use crossbeam_channel::{Sender, Receiver};
use std::hash::Hash;
use crate::pipeline::shader::ShaderAsset;
use crate::asset_storage::{ResourceLoadHandler, ResourceHandle};
use atelier_assets::core::AssetUuid;
use atelier_assets::loader::AssetLoadOp;
use crate::pipeline::pipeline::PipelineAsset;
use std::time::Duration;
use atelier_assets::loader::handle::AssetHandle;
use std::mem::swap;
use std::hint::unreachable_unchecked;
use crate::pipeline_description::SwapchainSurfaceInfo;
use crate::asset_resource::AssetResource;
use atelier_assets::loader::handle::Handle;



// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct PipelineResourceHash(u64);

impl PipelineResourceHash {
    pub fn from_resource_description<DscT : Hash>(t: &DscT) -> PipelineResourceHash {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        PipelineResourceHash(hasher.finish())
    }
}










struct PipelineResourceManagerHashState<GpuResourceT> {
    ref_count: std::sync::atomic::AtomicU32,
    vk_obj: GpuResourceT
}

// Contains a list of reference counted objects. It's like a HashMap<HashT, Arc<ResourceT>> except
// we don't want to rely on Drop to do the cleanup because we need a reference to the device to
// destroy the resource.
// - The caller must destroy the resource if remove_ref it
// -
pub struct DeduplicatedSet<HashT, DscT, ResourceT>
    where
        HashT: Eq + Hash,
        ResourceT: Clone,
{
    // Look up
    by_hash: FnvHashMap<HashT, PipelineResourceManagerHashState<ResourceT>>,

    // To detect collisions and so that we can recreate when swapchains are added/removed
    values: FnvHashMap<HashT, DscT>,
}


impl<HashT, DscT, ResourceT> Default for DeduplicatedSet<HashT, DscT, ResourceT>
    where
        HashT: Eq + Hash,
        ResourceT: Clone,
{
    fn default() -> Self {
        DeduplicatedSet {
            by_hash: Default::default(),
            values: Default::default(),
        }
    }
}

impl<HashT, DscT, ResourceT> DeduplicatedSet<HashT, DscT, ResourceT>
    where
        HashT: Eq + Hash,
        ResourceT: Clone,
{
    fn len(&self) -> usize {
        self.by_hash.len()
    }

    // Returns the resource
    fn get(
        &self,
        resource_hash: HashT
    ) -> Option<ResourceT> {
        self.by_hash.get(&resource_hash).map(|state| state.vk_obj.clone())
    }

    fn contains(
        &self,
        hash: HashT,
    ) -> bool {
        self.by_hash.contains_key(&hash)
    }

    fn insert(
        &mut self,
        hash: HashT,
        resource: ResourceT
    ) {
        // Insert the resource
        let old_object = self.by_hash.insert(hash, PipelineResourceManagerHashState {
            ref_count: std::sync::atomic::AtomicU32::new(1),
            vk_obj: resource
        });

        // If this trips, we have duplicate objects for the same load handle which isn't allowed.
        // The caller should have called add_ref instead
        assert!(old_object.is_none());
    }

    fn try_add_ref(
        &mut self,
        hash: HashT,
    ) -> Option<ResourceT> {
        // Add ref count
        //let state = self.by_hash.get(&hash);
        self.by_hash.get(&hash).map(|state| {
            state.ref_count.fetch_add(1, std::sync::atomic::Ordering::Acquire);
            state.vk_obj.clone()
        })
    }

    fn remove_ref(
        &mut self,
        hash: HashT
    ) -> Option<ResourceT>  {
        let ref_count = {
            // Decrement the ref count. Subtract 1 to the returned value because fetch_sub returns
            // it in the state before it was subtracted
            let state = self.by_hash.get(&hash).unwrap();
            state.ref_count.fetch_sub(1, std::sync::atomic::Ordering::Release) - 1
        };

        if ref_count == 0 {
            // Ref count is 0, return the underlying object since it's ready to be destroyed
            let state = self.by_hash.remove(&hash).unwrap();
            Some(state.vk_obj)
        } else {
            None
        }
    }

    // Intended for use when destroying so that resources can be cleaned up
    fn take_all_resources(&mut self) -> Vec<ResourceT> {
        let mut resources = Vec::with_capacity(self.by_hash.len());
        for (k, v) in &self.by_hash {
            resources.push(v.vk_obj.clone());
        }

        self.by_hash.clear();
        resources
    }

}

















// Contains a set of GPU resources, deduplicated by hashing.
// - Every insert has to be associated with an asset LoadHandle
// - If two load handles try to insert the same value, only one GPU resource is created/stored
// - Any insertion must be matched by a remove using the same LoadHandle
// - The final LoadHandle remove will send the resource to something that drops it via drop_sender

pub struct PipelineResourceManager<DscT : Hash, GpuResourceT : Clone> {
    // Look up the hash of a resource by load handle. The resource could be cached here but since
    // they are reference counted, we would need to lookup by the hash anyways (or wrap the ref
    // count in an arc)
    by_load_handle_committed: FnvHashMap<LoadHandle, PipelineResourceHash>,
    by_load_handle_uncommitted: FnvHashMap<LoadHandle, PipelineResourceHash>,

    // Look up
    //by_hash: FnvHashMap<PipelineResourceHash, PipelineResourceManagerHashState<GpuResourceT>>,
    by_hash: DeduplicatedSet<PipelineResourceHash, DscT, GpuResourceT>,

    // To detect collisions and so that we can recreate when swapchains are added/removed
    values: FnvHashMap<PipelineResourceHash, DscT>,

    phantom_data: PhantomData<DscT>
}

impl<DscT : PartialEq + Clone + Hash, GpuResourceT : Clone> Default for PipelineResourceManager<DscT, GpuResourceT> {
    fn default() -> Self {
        PipelineResourceManager {
            by_load_handle_committed: Default::default(),
            by_load_handle_uncommitted: Default::default(),
            by_hash: Default::default(),
            values: Default::default(),
            phantom_data: Default::default()
        }
    }
}

impl<DscT : PartialEq + Clone + Hash, GpuResourceT : Clone> PipelineResourceManager<DscT, GpuResourceT> {
    fn len(&self) -> usize {
        self.by_hash.len()
    }

    // Returns the resource
    fn get_committed(
        &self,
        load_handle: LoadHandle
    ) -> Option<GpuResourceT> {
        let resource_hash = self.by_load_handle_committed.get(&load_handle);
        resource_hash.and_then(|resource_hash| self.by_hash.get(*resource_hash))
    }

    fn get_latest(
        &self,
        load_handle: LoadHandle
    ) -> Option<GpuResourceT> {
        let resource_hash = self.by_load_handle_uncommitted.get(&load_handle);
        if let Some(resource_hash) = resource_hash {
            self.by_hash.get(*resource_hash)
        } else {
            self.get_committed(load_handle)
        }
    }

    fn contains_resource(
        &self,
        hash: PipelineResourceHash,
    ) -> bool {
        self.by_hash.contains(hash)
    }

    fn insert(
        &mut self,
        hash: PipelineResourceHash,
        load_handle: LoadHandle,
        dsc: &DscT,
        resource: GpuResourceT
    ) {
        debug_assert!(!self.contains_resource(hash));

        // Insert the resource
        self.by_hash.insert(hash, resource);

        let old_load_handle = self.by_load_handle_uncommitted.insert(load_handle, hash);
        // If this trips, we have duplicate objects for the same load handle
        assert!(old_load_handle.is_none());

        let old_value = self.values.insert(hash, dsc.clone());
        // If this trips, we incorrectly inserted a new value over a value that already existed
        assert!(old_value.is_none());
    }

    fn try_add_ref(
        &mut self,
        hash: PipelineResourceHash,
        load_handle: LoadHandle,
        dsc: &DscT,
    ) -> Option<GpuResourceT> {
        self.by_hash.try_add_ref(hash).map(|resource| {
            // Check the hash has no false collisions
            assert!(self.values.get(&hash).unwrap() == dsc);

            let old_load_handle = self.by_load_handle_uncommitted.insert(load_handle, hash);
            // Check that this load handle wasn't being used to load something else
            assert!(old_load_handle.is_none());
            resource
        })
    }

    fn remove_ref(
        &mut self,
        load_handle: LoadHandle,
    ) -> Vec<GpuResourceT> {
        // Worst-case we can have both a committed and uncommitted asset created
        let mut resources = Vec::with_capacity(2);
        if let Some(hash) = self.by_load_handle_committed.remove(&load_handle) {
            if let Some(resource) = self.by_hash.remove_ref(hash) {
                self.values.remove(&hash);
                resources.push(resource);
            }
        }

        if let Some(hash) = self.by_load_handle_uncommitted.remove(&load_handle) {
            if let Some(resource) = self.by_hash.remove_ref(hash) {
                self.values.remove(&hash);
                resources.push(resource);
            }
        }

        resources
    }

    fn commit(&mut self, load_handle: LoadHandle) {
        if let Some(hash) = self.by_load_handle_uncommitted.remove(&load_handle) {
            let old_load_handle = self.by_load_handle_committed.insert(load_handle, hash);
            assert!(old_load_handle.is_none());
        }
    }

    // Intended for use when destroying so that resources can be cleaned up
    fn take_all_resources(&mut self) -> Vec<GpuResourceT> {
        self.values.clear();
        self.by_load_handle_committed.clear();
        self.by_load_handle_uncommitted.clear();
        self.by_hash.take_all_resources()
    }
}









pub struct LoadRequest<T> {
    load_handle: LoadHandle,
    load_op: AssetLoadOp,
    description: T,
}

pub struct CommitRequest<T> {
    load_handle: LoadHandle,
    phantom_data: PhantomData<T>
}

pub struct FreeRequest<T> {
    load_handle: LoadHandle,
    phantom_data: PhantomData<T>
}

#[derive(Clone)]
pub struct LoadQueuesTx<T> {
    load_request_tx: Sender<LoadRequest<T>>,
    commit_request_tx: Sender<CommitRequest<T>>,
    free_request_tx: Sender<FreeRequest<T>>,
}

pub struct LoadQueuesRx<T> {
    load_request_rx: Receiver<LoadRequest<T>>,
    commit_request_rx: Receiver<CommitRequest<T>>,
    free_request_rx: Receiver<FreeRequest<T>>,
}

pub struct LoadQueues<T> {
    tx: LoadQueuesTx<T>,
    rx: LoadQueuesRx<T>,
}

impl<T> LoadQueues<T> {
    pub fn take_load_requests(&mut self) -> Vec<LoadRequest<T>> {
        self.rx.load_request_rx.try_iter().collect()
    }

    pub fn take_commit_requests(&mut self) -> Vec<CommitRequest<T>> {
        self.rx.commit_request_rx.try_iter().collect()
    }

    pub fn take_free_requests(&mut self) -> Vec<FreeRequest<T>> {
        self.rx.free_request_rx.try_iter().collect()
    }
}

impl<T> Default for LoadQueues<T> {
    fn default() -> Self {
        let (load_request_tx, load_request_rx) = crossbeam_channel::unbounded();
        let (commit_request_tx, commit_request_rx) = crossbeam_channel::unbounded();
        let (free_request_tx, free_request_rx) = crossbeam_channel::unbounded();

        let tx = LoadQueuesTx {
            load_request_tx,
            commit_request_tx,
            free_request_tx
        };

        let rx = LoadQueuesRx {
            load_request_rx,
            commit_request_rx,
            free_request_rx
        };

        LoadQueues {
            tx,
            rx
        }
    }
}

pub struct ActiveSwapchainSurfaceInfoState {
    ref_count: u32,
    index: usize
}

#[derive(Default)]
pub struct ActiveSwapchainSurfaceInfoSet {
    ref_counts: FnvHashMap<dsc::SwapchainSurfaceInfo, ActiveSwapchainSurfaceInfoState>,

    //TODO: Could make this a slab which would persist indexes across frames
    unique_swapchain_infos: Vec<dsc::SwapchainSurfaceInfo>
}

impl ActiveSwapchainSurfaceInfoSet {
    fn add(&mut self, swapchain_surface_info: &dsc::SwapchainSurfaceInfo) -> bool {
        match self.ref_counts.get_mut(swapchain_surface_info) {
            Some(state) => {
                state.ref_count += 1;
                false
            },
            None => {
                &self.ref_counts.insert(swapchain_surface_info.clone(), ActiveSwapchainSurfaceInfoState {
                    ref_count: 1,
                    index: self.unique_swapchain_infos.len()
                });

                self.unique_swapchain_infos.push(swapchain_surface_info.clone());
                true
            }
        }
    }

    fn remove(&mut self, swapchain_surface_info: &dsc::SwapchainSurfaceInfo) -> Option<usize> {
        match self.ref_counts.get_mut(swapchain_surface_info) {
            Some(state) => {
                if state.ref_count == 1 {
                    let removed_index = state.index;
                    self.ref_counts.remove(swapchain_surface_info);

                    for (x, mut y) in &mut self.ref_counts {
                        if y.index > removed_index {
                            y.index -= 1;
                        }
                    }
                    self.unique_swapchain_infos.swap_remove(removed_index);
                    Some(removed_index)
                } else {
                    None
                }
            },
            // If it doesn't exist, then a remove call was made before a matching add call
            None => unreachable!()
        }
    }

    pub fn unique_swapchain_infos(&self) -> &[dsc::SwapchainSurfaceInfo] {
        &self.unique_swapchain_infos
    }
}

#[derive(Clone)]
struct HashedRenderpass {
    // shader_modules: Vec<vk::ShaderModule>,
    // descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    // pipeline_layout: vk::PipelineLayout,

    // Per swapchain
    swapchain_renderpasses: Vec<vk::RenderPass>
}

#[derive(Clone)]
struct HashedPipelineLayout {
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    pipeline_layout: vk::PipelineLayout,
}

#[derive(Clone)]
struct HashedGraphicsPipeline {
    shader_modules: Vec<vk::ShaderModule>,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,

    // Per swapchain
    swapchain_renderpasses: Vec<vk::RenderPass>,
    swapchain_pipelines: Vec<vk::Pipeline>
}


pub struct PipelineInfo {
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    pipeline_layout: vk::PipelineLayout,
    renderpass: vk::RenderPass,
    pipeline: vk::Pipeline,
}

//TODO: Use hashes as keys instead of values (but maybe leave debug-only verification to make sure
// there aren't any hash collisions
pub struct PipelineManager {
    device_context: VkDeviceContext,
    active_swapchain_surface_infos: ActiveSwapchainSurfaceInfoSet,

    // Sets of resources
    shader_modules: PipelineResourceManager<dsc::ShaderModule, vk::ShaderModule>,
    descriptor_set_layouts: PipelineResourceManager<dsc::DescriptorSetLayout, vk::DescriptorSetLayout>,
    pipeline_layouts: PipelineResourceManager<dsc::PipelineLayout, HashedPipelineLayout>,

    // Per-swapchain resources
    renderpasses: PipelineResourceManager<dsc::RenderPass, HashedRenderpass>,
    graphics_pipelines: PipelineResourceManager<PipelineAsset, HashedGraphicsPipeline>,

    // Queues for incoming load requests
    shader_load_queues: LoadQueues<dsc::ShaderModule>,
    graphics_pipeline_load_queues: LoadQueues<PipelineAsset>
}

impl PipelineManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        PipelineManager {
            device_context: device_context.clone(),
            active_swapchain_surface_infos: Default::default(),
            shader_modules: Default::default(),
            descriptor_set_layouts: Default::default(),
            pipeline_layouts: Default::default(),
            renderpasses: Default::default(),
            graphics_pipelines: Default::default(),
            shader_load_queues: Default::default(),
            graphics_pipeline_load_queues: Default::default(),
        }
    }

    pub fn add_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo
    ) -> VkResult<()> {
        // Add it
        if self.active_swapchain_surface_infos.add(&swapchain_surface_info) {
            // Allocate a new renderpass instance for every loaded renderpass for the swapchain
            for (hash, state) in &mut self.renderpasses.by_hash.by_hash {
                let renderpass_dsc = self.renderpasses.values.get(hash).unwrap();
                let renderpasses = Self::create_renderpasses(
                    &self.device_context,
                    &[swapchain_surface_info.clone()],
                    renderpass_dsc
                )?;

                let new_renderpass = renderpasses[0];
                state.vk_obj.swapchain_renderpasses.push(new_renderpass);
            };

            // Allocate a new pipeline instance for every loaded pipeline for the swapchain
            for (hash, state) in &mut self.graphics_pipelines.by_hash.by_hash {
                let pipeline_asset = self.graphics_pipelines.values.get(hash).unwrap();

                let shader_modules = &state.vk_obj.shader_modules;
                let pipeline_layout = state.vk_obj.pipeline_layout;

                let renderpasses = &self.renderpasses.by_hash.get(PipelineResourceHash::from_resource_description(&pipeline_asset.renderpass)).unwrap();
                let new_renderpass = renderpasses.swapchain_renderpasses.last().unwrap();

                let pipeline = Self::create_graphics_pipelines(
                    &self.device_context,
                    shader_modules.as_slice(),
                    pipeline_layout,
                    &[swapchain_surface_info.clone()],
                    &[*new_renderpass],
                    &pipeline_asset
                )?;

                state.vk_obj.swapchain_pipelines.push(pipeline[0]);
            }
        }

        Ok(())

    }

    pub fn remove_swapchain(&mut self, swapchain_surface_info: &dsc::SwapchainSurfaceInfo) {
        // Remove it
        if let Some(removed_index) = self.active_swapchain_surface_infos.remove(&swapchain_surface_info) {
            // If necessary, destroy some resources
            for (hash, state) in &mut self.renderpasses.by_hash.by_hash {
                let renderpass = state.vk_obj.swapchain_renderpasses.swap_remove(removed_index);
                //TODO: Free renderpass after a few frames
            }

            for (hash, state) in &mut self.graphics_pipelines.by_hash.by_hash {
                let pipeline = state.vk_obj.swapchain_pipelines.swap_remove(removed_index);
                //TODO: Free pipeline after a few frames
            }
        }

        //TODO: Common case is to destroy and re-create the same swapchain surface info, so we can
        // delay destroying until we also get an additional add/remove. If the next add call is
        // the same, we can avoid the remove entirely
    }

    pub fn update(&mut self) {
        let shader_module_count = self.shader_modules.len();
        let descriptor_set_layout_count = self.descriptor_set_layouts.len();
        let pipeline_layout_count = self.pipeline_layouts.len();
        let renderpass_count = self.renderpasses.len();
        let pipeline_count = self.graphics_pipelines.len();

        #[derive(Debug)]
        struct ResourceCounts {
            shader_module_count: usize,
            descriptor_set_layout_count: usize,
            pipeline_layout_count: usize,
            renderpass_count: usize,
            pipeline_count: usize,
        }

        let resource_counts = ResourceCounts {
            shader_module_count,
            descriptor_set_layout_count,
            pipeline_layout_count,
            renderpass_count,
            pipeline_count,
        };

        //println!("Resource counts: {:#?}", resource_counts);

        self.process_shader_load_requests();
        self.process_pipeline_load_requests();
    }

    pub fn get_pipeline_info(&self, handle: &Handle<PipelineAsset>, swapchain: &SwapchainSurfaceInfo) -> PipelineInfo {
        let resource = self.graphics_pipelines.get_committed(handle.load_handle()).unwrap();
        let swapchain_index = self.active_swapchain_surface_infos.ref_counts.get(swapchain).unwrap().index;

        PipelineInfo {
            descriptor_set_layouts: resource.descriptor_set_layouts.clone(),
            pipeline_layout: resource.pipeline_layout,
            renderpass: resource.swapchain_renderpasses[swapchain_index],
            pipeline: resource.swapchain_pipelines[swapchain_index]
        }
    }

    fn process_shader_load_requests(&mut self) {
        for request in self.shader_load_queues.take_load_requests() {
            let result = self.load_shader_module(
                request.load_handle,
                &request.description
            );

            match result {
                Ok(_) => request.load_op.complete(),
                Err(err) => {
                    request.load_op.error(err);
                }
            }
        }

        for request in self.shader_load_queues.take_commit_requests() {
            self.shader_modules.commit(request.load_handle);
        }

        for request in self.shader_load_queues.take_free_requests() {
            for shader_module in self.shader_modules.remove_ref(request.load_handle) {
                unsafe {
                    //TODO: May need to push into a queue to destroy later
                    //self.device_context.device().destroy_shader_module(shader_module);
                }
            }
        }
    }

    fn process_pipeline_load_requests(&mut self) {
        for request in self.graphics_pipeline_load_queues.take_load_requests() {
            let result = self.load_graphics_pipeline(
                request.load_handle,
                &request.description
            );

            match result {
                Ok(_) => request.load_op.complete(),
                Err(err) => {
                    //TODO: May need to unregister upstream dependencies (like shaders, pipeline layouts, descriptor sets)
                    request.load_op.error(err);
                }
            }
        }

        for request in self.graphics_pipeline_load_queues.take_commit_requests() {
            // Have to move the commit references for all upstream things
            self.shader_modules.commit(request.load_handle);
            self.descriptor_set_layouts.commit(request.load_handle);
            self.pipeline_layouts.commit(request.load_handle);
            self.renderpasses.commit(request.load_handle);
            self.graphics_pipelines.commit(request.load_handle);
        }

        for request in self.graphics_pipeline_load_queues.take_free_requests() {
            for graphics_pipeline in self.shader_modules.remove_ref(request.load_handle) {
                unsafe {
                    //TODO: May need to push into a queue to destroy later
                    //self.device_context.device().destroy_shader_module(shader_module);
                }
            }
        }
    }

    pub fn create_shader_load_handler(&self) -> ShaderLoadHandler {
        ShaderLoadHandler {
            load_queues: self.shader_load_queues.tx.clone()
        }
    }

    pub fn create_pipeline_load_handler(&self) -> PipelineLoadHandler {
        PipelineLoadHandler {
            load_queues: self.graphics_pipeline_load_queues.tx.clone()
        }
    }

    fn load_shader_module(
        &mut self,
        load_handle: LoadHandle,
        shader_module: &dsc::ShaderModule,
    ) -> VkResult<vk::ShaderModule> {
        //TODO: Might be more consistent to hash the shader asset?
        let hash = PipelineResourceHash::from_resource_description(shader_module);
        if let Some(resource) = self.shader_modules.try_add_ref(hash, load_handle, shader_module) {
            //Ok(self.shader_modules.add_ref_by_hash(hash, load_handle, shader_module))
            Ok(resource)
        } else {
            println!("Creating shader module\n[bytes: {}]", shader_module.code.len());
            let resource =
                crate::pipeline_description::create_shader_module(self.device_context.device(), shader_module)?;
            self.shader_modules.insert(hash, load_handle, shader_module, resource);
            Ok(resource)
        }
    }

    fn load_descriptor_set_layout(
        &mut self,
        load_handle: LoadHandle,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<vk::DescriptorSetLayout> {
        let hash = PipelineResourceHash::from_resource_description(descriptor_set_layout);
        if let Some(resource) = self.descriptor_set_layouts.try_add_ref(hash, load_handle, descriptor_set_layout) {
            //Ok(self.descriptor_set_layouts.add_ref_by_hash(hash, load_handle, descriptor_set_layout))
            Ok(resource)
        } else {
            println!("Creating descriptor set layout\n{:#?}", descriptor_set_layout);
            let resource =
                crate::pipeline_description::create_descriptor_set_layout(self.device_context.device(), descriptor_set_layout)?;
            self.descriptor_set_layouts.insert(hash, load_handle, descriptor_set_layout, resource);
            Ok(resource)
        }
    }

    fn load_pipeline_layout(
        &mut self,
        load_handle: LoadHandle,
        pipeline_layout: &dsc::PipelineLayout
    ) -> VkResult<HashedPipelineLayout> {
        let hash = PipelineResourceHash::from_resource_description(pipeline_layout);
        if let Some(resource) = self.pipeline_layouts.try_add_ref(hash, load_handle, pipeline_layout) {
            //Ok(self.pipeline_layouts.add_ref_by_hash(hash, load_handle, pipeline_layout))
            Ok(resource)
        } else {
            let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout.descriptor_set_layouts.len());
            for descriptor_set_layout in &pipeline_layout.descriptor_set_layouts {
                descriptor_set_layouts.push(self.load_descriptor_set_layout(load_handle, descriptor_set_layout)?);
            }

            println!("Creating pipeline layout\n{:#?}", pipeline_layout);
            let resource =
                crate::pipeline_description::create_pipeline_layout(self.device_context.device(), pipeline_layout, &descriptor_set_layouts)?;

            let hashed_pipeline_layout = HashedPipelineLayout {
                pipeline_layout: resource,
                descriptor_set_layouts: descriptor_set_layouts
            };

            self.pipeline_layouts.insert(hash, load_handle, pipeline_layout, hashed_pipeline_layout.clone());
            Ok(hashed_pipeline_layout)
        }
    }

    fn create_renderpasses(
        device_context: &VkDeviceContext,
        swapchain_infos: &[SwapchainSurfaceInfo],
        renderpass: &dsc::RenderPass,
    ) -> VkResult<Vec<vk::RenderPass>> {
        println!("Creating renderpasses\n{:#?}", renderpass);
        let mut resources = Vec::with_capacity(swapchain_infos.len());
        for swapchain_info in swapchain_infos {
            let resource =
                crate::pipeline_description::create_renderpass(device_context.device(), renderpass, swapchain_info)?;
            resources.push(resource);
        }

        Ok(resources)
    }


    fn load_renderpass(
        &mut self,
        load_handle: LoadHandle,
        renderpass: &dsc::RenderPass,
    ) -> VkResult<HashedRenderpass> {
        let hash = PipelineResourceHash::from_resource_description(renderpass);
        if let Some(resource) = self.renderpasses.try_add_ref(hash, load_handle, renderpass) {
            //Ok(self.renderpasses.add_ref_by_hash(hash, load_handle, renderpass))
            Ok(resource)
        } else {
            // println!("Creating renderpasses\n{:#?}", renderpass);
            // let mut resources = Vec::with_capacity(self.active_swapchain_surface_infos.unique_swapchain_infos().len());
            // for swapchain_info in self.active_swapchain_surface_infos.unique_swapchain_infos() {
            //     let resource =
            //         crate::pipeline_description::create_renderpass(self.device_context.device(), renderpass, swapchain_info)?;
            //     resources.push(resource);
            // }

            let resources = Self::create_renderpasses(
                &self.device_context,
                &self.active_swapchain_surface_infos.unique_swapchain_infos(),
                renderpass
            )?;

            let hashed_renderpass = HashedRenderpass {
                swapchain_renderpasses: resources
            };

            self.renderpasses.insert(hash, load_handle, renderpass, hashed_renderpass.clone());
            Ok(hashed_renderpass)
        }
    }

    fn create_graphics_pipelines(
        device_context: &VkDeviceContext,
        shader_modules: &[vk::ShaderModule],
        pipeline_layout: vk::PipelineLayout,
        swapchain_infos: &[SwapchainSurfaceInfo],
        renderpasses: &[vk::RenderPass],
        graphics_pipeline: &PipelineAsset,
    ) -> VkResult<Vec<vk::Pipeline>> {
        //let pipeline_layout = self.load_pipeline_layout(load_handle, &graphics_pipeline.pipeline_layout)?;
        //let renderpasses = self.load_renderpass(load_handle, &graphics_pipeline.renderpass)?;

        let mut shader_modules_meta = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
        //let mut shader_modules = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
        for stage in &graphics_pipeline.pipeline_shader_stages {
            //let shader_module = self.shader_modules.get_latest(stage.shader_module.load_handle()).unwrap();
            let shader_module_meta = dsc::ShaderModuleMeta {
                stage: stage.stage,
                entry_name: stage.entry_name.clone()
            };
            //shader_modules.push(shader_module);
            shader_modules_meta.push(shader_module_meta);
        }

        println!("Creating graphics pipeline\n{:#?}\n{:#?}", graphics_pipeline.fixed_function_state, shader_modules_meta);

        let mut resources = Vec::with_capacity(swapchain_infos.len());
        for (swapchain_surface_info, renderpass) in swapchain_infos.iter().zip(renderpasses) {
            let resource =
                crate::pipeline_description::create_graphics_pipeline(
                    device_context.device(),
                    &graphics_pipeline.fixed_function_state,
                    pipeline_layout,
                    *renderpass,
                    &shader_modules_meta,
                    &shader_modules,
                    swapchain_surface_info
                )?;
            resources.push(resource);
        }

        Ok(resources)
    }

    fn load_graphics_pipeline(
        &mut self,
        load_handle: LoadHandle,
        graphics_pipeline: &PipelineAsset,
    ) -> VkResult<HashedGraphicsPipeline> {
        //TODO: Hashing the asset comes with the downside that if multiple shader assets are the same, we don't deduplicate them.
        let hash = PipelineResourceHash::from_resource_description(graphics_pipeline);
        if let Some(resource) = self.graphics_pipelines.try_add_ref(hash, load_handle, graphics_pipeline) {
            //Ok(self.graphics_pipelines.add_ref_by_hash(hash, load_handle, graphics_pipeline).swapchain_pipelines.clone())
            Ok(resource)
        } else {
            let pipeline_layout = self.load_pipeline_layout(load_handle, &graphics_pipeline.pipeline_layout)?;
            let renderpasses = self.load_renderpass(load_handle, &graphics_pipeline.renderpass)?;

            // let mut shader_modules_meta = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
            let mut shader_modules = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
            for stage in &graphics_pipeline.pipeline_shader_stages {
                let shader_module = self.shader_modules.get_latest(stage.shader_module.load_handle()).unwrap();
                shader_modules.push(shader_module);
            }

            //let resources = Self::create_renderpasses(&self.device_context, load_handle, &self.active_swapchain_surface_infos.unique_swapchain_infos(), renderpass)?;
            let resources = Self::create_graphics_pipelines(
                &self.device_context,
                &shader_modules,
                pipeline_layout.pipeline_layout,
                &self.active_swapchain_surface_infos.unique_swapchain_infos,
                &renderpasses.swapchain_renderpasses,
                graphics_pipeline
            )?;

            let hashed_graphics_pipeline = HashedGraphicsPipeline {
                shader_modules,
                descriptor_set_layouts: pipeline_layout.descriptor_set_layouts,
                pipeline_layout: pipeline_layout.pipeline_layout,
                swapchain_renderpasses: renderpasses.swapchain_renderpasses.clone(),
                swapchain_pipelines: resources.clone()
            };

            self.graphics_pipelines.insert(hash, load_handle, graphics_pipeline, hashed_graphics_pipeline.clone());
            Ok(hashed_graphics_pipeline)
        }
    }
}

impl Drop for PipelineManager {
    fn drop(&mut self) {
        unsafe {
            for resources in self.graphics_pipelines.take_all_resources() {
                for resource in resources.swapchain_pipelines {
                    self.device_context.device().destroy_pipeline(resource, None);
                }
            }

            for resources in self.renderpasses.take_all_resources() {
                for resource in resources.swapchain_renderpasses {
                    self.device_context.device().destroy_render_pass(resource, None);
                }
            }

            for resource in self.pipeline_layouts.take_all_resources() {
                self.device_context.device().destroy_pipeline_layout(resource.pipeline_layout, None);
            }

            for resource in self.descriptor_set_layouts.take_all_resources() {
                self.device_context.device().destroy_descriptor_set_layout(resource, None);
            }

            for resource in self.shader_modules.take_all_resources() {
                self.device_context.device().destroy_shader_module(resource, None);
            }
        }
    }
}


pub struct ShaderLoadHandler {
    load_queues: LoadQueuesTx<dsc::ShaderModule>
}

impl ResourceLoadHandler<ShaderAsset> for ShaderLoadHandler {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<ShaderAsset>,
        version: u32,
        asset: &ShaderAsset,
        load_op: AssetLoadOp
    ) {
        let description = dsc::ShaderModule {
            code: asset.data.clone()
        };

        let request = LoadRequest {
            load_handle,
            load_op,
            description
        };

        self.load_queues.load_request_tx.send(request);
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<ShaderAsset>,
        version: u32,
        asset: &ShaderAsset
    ) {
        let request = CommitRequest {
            load_handle,
            phantom_data: Default::default()
        };

        self.load_queues.commit_request_tx.send(request);
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<ShaderAsset>,
        version: u32,
    ) {
        let request = FreeRequest {
            load_handle,
            phantom_data: Default::default()
        };

        self.load_queues.free_request_tx.send(request);
    }
}

pub struct PipelineLoadHandler {
    load_queues: LoadQueuesTx<PipelineAsset>
}

impl ResourceLoadHandler<PipelineAsset> for PipelineLoadHandler {
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<PipelineAsset>,
        version: u32,
        asset: &PipelineAsset,
        load_op: AssetLoadOp
    ) {
        let description = asset.clone();

        let request = LoadRequest {
            load_handle,
            load_op,
            description
        };

        self.load_queues.load_request_tx.send(request);
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<PipelineAsset>,
        version: u32,
        asset: &PipelineAsset
    ) {
        let request = CommitRequest {
            load_handle,
            phantom_data: Default::default()
        };

        self.load_queues.commit_request_tx.send(request);
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<PipelineAsset>,
        version: u32,
    ) {
        let request = FreeRequest {
            load_handle,
            phantom_data: Default::default()
        };

        self.load_queues.free_request_tx.send(request);
    }
}