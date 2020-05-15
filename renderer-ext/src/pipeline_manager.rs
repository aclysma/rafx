
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

// #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
// pub struct LoadHandleVersion {
//     load_handle: LoadHandle,
//     version: u32,
// }
//
// impl LoadHandleVersion {
//     pub fn new(
//         load_handle: LoadHandle,
//         version: u32,
//     ) -> Self {
//         LoadHandleVersion {
//             load_handle,
//             version,
//         }
//     }
// }

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct PipelineResourceHash(u64);

// Contains a set of GPU resources, deduplicated by hashing.
// - Every insert has to be associated with an asset LoadHandle
// - If two load handles try to insert the same value, only one GPU resource is created/stored
// - Any insertion must be matched by a remove using the same LoadHandle
// - The final LoadHandle remove will send the resource to something that drops it via drop_sender
struct PipelineResourceManagerHashState<GpuResourceT> {
    ref_count: std::sync::atomic::AtomicU32,
    vk_obj: GpuResourceT
}

pub struct PipelineResourceManager<DscT, GpuResourceT : Copy> {
    // Look up the hash of a resource by load handle. The resource could be cached here but since
    // they are reference counted, we would need to lookup by the hash anyways (or wrap the ref
    // count in an arc)
    by_load_handle_committed: FnvHashMap<LoadHandle, PipelineResourceHash>,
    by_load_handle_uncommitted: FnvHashMap<LoadHandle, PipelineResourceHash>,

    // Look up
    by_hash: FnvHashMap<PipelineResourceHash, PipelineResourceManagerHashState<GpuResourceT>>,

    // For debug purposes only to detect collisions
    values: FnvHashMap<PipelineResourceHash, DscT>,

    phantom_data: PhantomData<DscT>
}

impl<DscT : PartialEq + Clone, GpuResourceT : Copy> Default for PipelineResourceManager<DscT, GpuResourceT> {
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

impl<DscT : PartialEq + Clone, GpuResourceT : Copy> PipelineResourceManager<DscT, GpuResourceT> {
    // Returns the resource
    fn get_committed(
        &self,
        load_handle: LoadHandle
    ) -> Option<GpuResourceT> {
        let resource_hash = self.by_load_handle_committed.get(&load_handle);
        resource_hash.and_then(|resource_hash| self.get_internal(*resource_hash))
    }

    fn get_latest(
        &self,
        load_handle: LoadHandle
    ) -> Option<GpuResourceT> {
        let resource_hash = self.by_load_handle_uncommitted.get(&load_handle);
        if let Some(resource_hash) = resource_hash {
            self.get_internal(*resource_hash)
        } else {
            self.get_committed(load_handle)
        }
    }

    // Returns the resource
    fn get_internal(
        &self,
        resource_hash: PipelineResourceHash
    ) -> Option<GpuResourceT> {
        self.by_hash.get(&resource_hash).map(|state| state.vk_obj)
    }

    fn contains_resource(
        &self,
        hash: PipelineResourceHash,
    ) -> bool {
        self.by_hash.contains_key(&hash)
    }

    fn insert(
        &mut self,
        hash: PipelineResourceHash,
        load_handle: LoadHandle,
        dsc: &DscT,
        resource: GpuResourceT
    ) -> VkResult<GpuResourceT> {
        debug_assert!(!self.contains_resource(hash));

        // Insert the resource
        self.by_hash.insert(hash, PipelineResourceManagerHashState {
            ref_count: std::sync::atomic::AtomicU32::new(1),
            vk_obj: resource
        });

        self.by_load_handle_uncommitted.insert(load_handle, hash);
        self.values.insert(hash, dsc.clone());
        Ok(resource)
    }

    fn add_ref_by_hash(
        &mut self,
        hash: PipelineResourceHash,
        load_handle: LoadHandle,
        dsc: &DscT,
    ) -> GpuResourceT {
        // Check the hash has no false collisions
        if let Some(value) = self.values.get(&hash) {
            assert!(*dsc == *value);
        } else {
            self.values.insert(hash, dsc.clone());
        }

        self.add_ref_internal(load_handle, hash)
    }

    fn add_ref_by_handle<T>(
        &mut self,
        load_handle: LoadHandle,
        existing_asset_handle: &atelier_assets::loader::handle::Handle<T>,
    ) -> GpuResourceT {
        use atelier_assets::loader::handle::AssetHandle;
        // Find the hash the existing asset is loaded under
        let hash = if let Some(x) = self.by_load_handle_uncommitted.get(&existing_asset_handle.load_handle()) {
            Some(*x)
        } else if let Some(x) = self.by_load_handle_committed.get(&existing_asset_handle.load_handle()) {
            Some(*x)
        } else {
            None
        };

        self.add_ref_internal(load_handle, hash.unwrap())
    }

    fn add_ref_internal(
        &mut self,
        load_handle: LoadHandle,
        hash: PipelineResourceHash,
    ) -> GpuResourceT {
        // Add ref count
        let state = self.by_hash.get(&hash).unwrap();
        state.ref_count.fetch_add(1, std::sync::atomic::Ordering::Acquire);

        // Store the load handle
        self.by_load_handle_uncommitted.insert(load_handle, hash);
        state.vk_obj
    }

    fn remove_ref(
        &mut self,
        load_handle: LoadHandle,
    ) -> Vec<GpuResourceT> {
        // Worst-case we can have both a committed and uncommitted asset created
        let mut resources = Vec::with_capacity(2);
        if let Some(hash) = self.by_load_handle_committed.remove(&load_handle) {
            if let Some(resource) = self.remove_ref_internal(load_handle, hash) {
                resources.push(resource);
            }
        }

        if let Some(hash) = self.by_load_handle_uncommitted.remove(&load_handle) {
            if let Some(resource) = self.remove_ref_internal(load_handle, hash) {
                resources.push(resource);
            }
        }

        resources
    }

    fn remove_ref_internal(
        &mut self,
        load_handle: LoadHandle,
        hash: PipelineResourceHash
    ) -> Option<GpuResourceT>  {
        let (ref_count, vk_obj) = {
            let state = self.by_hash.get(&hash).unwrap();

            // Subtract one because fetch_sub returns the value in the state before it was subtracted
            let ref_count = state.ref_count.fetch_sub(1, std::sync::atomic::Ordering::Release) - 1;
            (ref_count, state.vk_obj)
        };

        if ref_count == 0 {
            self.values.remove(&hash);
            self.by_hash.remove(&hash);

            // Return the underlying object if it is ready to be destroyed
            Some(vk_obj)
        } else {
            None
        }
    }

    // Intended for use when destroying so that resources can be cleaned up
    fn take_all_resources(&mut self) -> Vec<GpuResourceT> {
        let resources = self.by_hash.iter().map(|(_, v)| v.vk_obj).collect();
        self.by_hash.clear();
        self.values.clear();
        self.by_load_handle_committed.clear();
        self.by_load_handle_uncommitted.clear();
        resources
    }
}

fn hash_resource_description<T : Hash>(t: &T) -> PipelineResourceHash {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    PipelineResourceHash(hasher.finish())
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
        let mut requests = Vec::with_capacity(self.rx.load_request_rx.len());
        while let Ok(request) = self.rx.load_request_rx.recv_timeout(Duration::from_secs(0)) {
            requests.push(request);
        }

        requests
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



//TODO: Use hashes as keys instead of values (but maybe leave debug-only verification to make sure
// there aren't any hash collisions
pub struct PipelineManager {
    device_context: VkDeviceContext,
    swapchain_surface_info: Option<dsc::SwapchainSurfaceInfo>,

    // Sets of resources
    shader_modules: PipelineResourceManager<dsc::ShaderModule, vk::ShaderModule>,
    descriptor_set_layouts: PipelineResourceManager<dsc::DescriptorSetLayout, vk::DescriptorSetLayout>,
    pipeline_layouts: PipelineResourceManager<dsc::PipelineLayout, vk::PipelineLayout>,
    renderpasses: PipelineResourceManager<dsc::RenderPass, vk::RenderPass>,
    graphics_pipelines: PipelineResourceManager<PipelineAsset, vk::Pipeline>,

    // Queues for incoming load requests
    shader_load_queues: LoadQueues<dsc::ShaderModule>,
    graphics_pipeline_load_queues: LoadQueues<PipelineAsset>
}

impl PipelineManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        PipelineManager {
            device_context: device_context.clone(),
            swapchain_surface_info: None,
            shader_modules: Default::default(),
            descriptor_set_layouts: Default::default(),
            pipeline_layouts: Default::default(),
            renderpasses: Default::default(),
            graphics_pipelines: Default::default(),
            shader_load_queues: Default::default(),
            graphics_pipeline_load_queues: Default::default(),
        }
    }

    pub fn update_swapchain_surface_info(&mut self, swapchain_surface_info: dsc::SwapchainSurfaceInfo) {
        self.swapchain_surface_info = Some(swapchain_surface_info);
    }

    pub fn swapchain_surface_info(&self) -> Option<&dsc::SwapchainSurfaceInfo> {
        self.swapchain_surface_info.as_ref()
    }
    //
    // pub fn shader_load_queues_tx(&self) -> &LoadQueuesTx<dsc::ShaderModule> {
    //     &self.shader_load_queues.tx
    // }

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

    pub fn load_shader_module(
        &mut self,
        load_handle: LoadHandle,
        shader_module: &dsc::ShaderModule,
    ) -> VkResult<vk::ShaderModule> {
        //TODO: Might be more consistent to hash the shader asset?
        let hash = hash_resource_description(&shader_module);
        if self.shader_modules.contains_resource(hash) {
            Ok(self.shader_modules.add_ref_by_hash(hash, load_handle, shader_module))
        } else {
            println!("Creating shader module\n[bytes: {}]", shader_module.code.len());
            let resource =
                crate::pipeline_description::create_shader_module(self.device_context.device(), shader_module)?;
            self.shader_modules.insert(hash, load_handle, shader_module, resource);
            Ok(resource)
        }
    }

    pub fn load_descriptor_set_layout(
        &mut self,
        load_handle: LoadHandle,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<vk::DescriptorSetLayout> {
        let hash = hash_resource_description(&descriptor_set_layout);
        if self.descriptor_set_layouts.contains_resource(hash) {
            Ok(self.descriptor_set_layouts.add_ref_by_hash(hash, load_handle, descriptor_set_layout))
        } else {
            println!("Creating descriptor set layout\n{:#?}", descriptor_set_layout);
            let resource =
                crate::pipeline_description::create_descriptor_set_layout(self.device_context.device(), descriptor_set_layout)?;
            self.descriptor_set_layouts.insert(hash, load_handle, descriptor_set_layout, resource);
            Ok(resource)
        }
    }

    pub fn load_pipeline_layout(
        &mut self,
        load_handle: LoadHandle,
        pipeline_layout: &dsc::PipelineLayout
    ) -> VkResult<vk::PipelineLayout> {

        let hash = hash_resource_description(&pipeline_layout);
        if self.pipeline_layouts.contains_resource(hash) {
            Ok(self.pipeline_layouts.add_ref_by_hash(hash, load_handle, pipeline_layout))
        } else {
            let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout.descriptor_set_layouts.len());
            for descriptor_set_layout in &pipeline_layout.descriptor_set_layouts {
                descriptor_set_layouts.push(self.load_descriptor_set_layout(load_handle, descriptor_set_layout)?);
            }

            println!("Creating pipeline layout\n{:#?}", pipeline_layout);
            let resource =
                crate::pipeline_description::create_pipeline_layout(self.device_context.device(), pipeline_layout, &descriptor_set_layouts)?;
            self.pipeline_layouts.insert(hash, load_handle, pipeline_layout, resource);
            Ok(resource)
        }
    }

    pub fn load_renderpass(
        &mut self,
        load_handle: LoadHandle,
        renderpass: &dsc::RenderPass,
    ) -> VkResult<vk::RenderPass> {
        let hash = hash_resource_description(&renderpass);
        if self.renderpasses.contains_resource(hash) {
            Ok(self.renderpasses.add_ref_by_hash(hash, load_handle, renderpass))
        } else {
            println!("Creating renderpass\n{:#?}", renderpass);
            let resource =
                crate::pipeline_description::create_renderpass(self.device_context.device(), renderpass, self.swapchain_surface_info.as_ref().unwrap())?;
            self.renderpasses.insert(hash, load_handle, renderpass, resource);
            Ok(resource)
        }
    }

    pub fn load_graphics_pipeline(
        &mut self,
        load_handle: LoadHandle,
        graphics_pipeline: &PipelineAsset,
    ) -> VkResult<vk::Pipeline> {
        let hash = hash_resource_description(&graphics_pipeline);
        if self.graphics_pipelines.contains_resource(hash) {
            Ok(self.graphics_pipelines.add_ref_by_hash(hash, load_handle, graphics_pipeline))
        } else {
            let pipeline_layout = self.load_pipeline_layout(load_handle, &graphics_pipeline.pipeline_layout)?;
            let renderpass = self.load_renderpass(load_handle, &graphics_pipeline.renderpass)?;

            let mut shader_modules_meta = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
            let mut shader_modules = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
            for stage in &graphics_pipeline.pipeline_shader_stages {
                let shader_module = self.shader_modules.get_latest(stage.shader_module.load_handle()).unwrap();
                let shader_module_meta = dsc::ShaderModuleMeta {
                    stage: stage.stage,
                    entry_name: stage.entry_name.clone()
                };
                shader_modules.push(shader_module);
                shader_modules_meta.push(shader_module_meta);
            }

            println!("Creating graphics pipeline\n{:#?}\n{:#?}", graphics_pipeline.fixed_function_state, shader_modules_meta);
            let resource =
                crate::pipeline_description::create_graphics_pipeline(
                    self.device_context.device(),
                    &graphics_pipeline.fixed_function_state,
                    pipeline_layout,
                    renderpass,
                    &shader_modules_meta,
                    &shader_modules,
                    self.swapchain_surface_info.as_ref().unwrap()
                )?;
            self.graphics_pipelines.insert(hash, load_handle, graphics_pipeline, resource);
            Ok(resource)
        }
    }

    pub fn update(&mut self) {
        let shader_module_count = self.shader_modules.by_hash.len();
        let descriptor_set_layout_count = self.descriptor_set_layouts.by_hash.len();
        let pipeline_layout_count = self.pipeline_layouts.by_hash.len();
        let renderpass_count = self.renderpasses.by_hash.len();
        let pipeline_count = self.graphics_pipelines.by_hash.len();

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

        println!("Resource counts: {:#?}", resource_counts);


        for request in self.shader_load_queues.take_load_requests() {
            let result = self.load_shader_module(request.load_handle, &request.description);
            match result {
                Ok(_) => request.load_op.complete(),
                Err(err) => {
                    //TODO: May need to unregister dependency assets
                    request.load_op.error(err);
                }
            }
        }

        // Delay processing pipelines until we have a swapchain - we can't init pipelines if we don't know the
        // window size/format
        if self.swapchain_surface_info.is_some() {
            for request in self.graphics_pipeline_load_queues.take_load_requests() {
                // let mut shaders = Vec::with_capacity(request.description.pipeline_shader_stages.len());
                // for shader_stage in &request.description.pipeline_shader_stages {
                //     let shader = self.shader_modules.get_latest(shader_stage.shader_module.load_handle());
                //     shaders.push(shader.unwrap());
                // }
                //
                // let graphics_pipeline = dsc::GraphicsPipeline {
                //     pipeline_layout: request.description.pipeline_layout,
                //     renderpass: request.description.renderpass,
                //     fixed_function_state: request.description.fixed_function_state,
                //     //pipeline_shader_stages: request.description.pipeline_shader_stages
                // };

                let result = self.load_graphics_pipeline(
                    request.load_handle,
                    &request.description
                );

                match result {
                    Ok(_) => request.load_op.complete(),
                    Err(err) => {
                        //TODO: May need to unregister dependency assets
                        request.load_op.error(err);
                    }
                }
            }
        }
    }
}

impl Drop for PipelineManager {
    fn drop(&mut self) {
        unsafe {
            for resource in self.graphics_pipelines.take_all_resources() {
                self.device_context.device().destroy_pipeline(resource, None);
            }

            for resource in self.renderpasses.take_all_resources() {
                self.device_context.device().destroy_render_pass(resource, None);
            }

            for resource in self.pipeline_layouts.take_all_resources() {
                self.device_context.device().destroy_pipeline_layout(resource, None);
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