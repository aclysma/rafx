
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LoadHandleVersion {
    load_handle: LoadHandle,
    version: u32,
}

impl LoadHandleVersion {
    pub fn new(
        load_handle: LoadHandle,
        version: u32,
    ) -> Self {
        LoadHandleVersion {
            load_handle,
            version,
        }
    }
}

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

#[derive(Default)]
pub struct PipelineResourceManager<DscT, GpuResourceT : Copy> {
    // Look up the hash of a resource by load handle. The resource could be cached here but since
    // they are reference counted, we would need to lookup by the hash anyways (or wrap the ref
    // count in an arc)
    by_load_handle: FnvHashMap<LoadHandleVersion, PipelineResourceHash>,

    // Look up
    by_hash: FnvHashMap<PipelineResourceHash, PipelineResourceManagerHashState<GpuResourceT>>,

    // For debug purposes only to detect collisions
    values: FnvHashMap<PipelineResourceHash, DscT>,

    current_version: FnvHashMap<LoadHandle, LoadHandleVersion>,

    phantom_data: PhantomData<DscT>
}

impl<DscT : PartialEq + Clone, GpuResourceT : Copy> PipelineResourceManager<DscT, GpuResourceT> {
    // Returns the resource
    fn get(
        &self,
        load_handle: LoadHandleVersion
    ) -> Option<GpuResourceT> {
        let resource_hash = self.by_load_handle.get(&load_handle);
        if let Some(resource_hash) = resource_hash {
            let resource_hash = *resource_hash;
            let state = self.by_hash.get(&resource_hash).unwrap();
            Some(state.vk_obj)
        } else {
            None
        }
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
        load_handle: LoadHandleVersion,
        dsc: &DscT,
        resource: GpuResourceT
    ) -> VkResult<GpuResourceT> {
        debug_assert!(!self.contains_resource(hash));

        // Insert the resource
        self.by_hash.insert(hash, PipelineResourceManagerHashState {
            ref_count: std::sync::atomic::AtomicU32::new(1),
            vk_obj: resource
        });

        self.by_load_handle.insert(load_handle, hash);
        self.values.insert(hash, dsc.clone());
        Ok(resource)
    }

    fn add_ref(
        &mut self,
        hash: PipelineResourceHash,
        load_handle: LoadHandleVersion,
        dsc: &DscT,
    ) -> GpuResourceT {
        // Add ref count
        let state = self.by_hash.get(&hash).unwrap();
        state.ref_count.fetch_add(1, std::sync::atomic::Ordering::Acquire);

        // Store the load handle
        self.by_load_handle.insert(load_handle, hash);

        if let Some(value) = self.values.get(&hash) {
            assert!(*dsc == *value);
        } else {
            self.values.insert(hash, dsc.clone());
        }

        state.vk_obj
    }

    fn remove_ref(
        &mut self,
        load_handle: LoadHandleVersion,
    ) -> Option<GpuResourceT> {
        match self.by_load_handle.get(&load_handle) {
            Some(hash) => {
                let hash = *hash;
                let (ref_count, vk_obj) = {
                    let state = self.by_hash.get(&hash).unwrap();

                    // Subtract one because fetch_sub returns the value in the state before it was subtracted
                    let ref_count = state.ref_count.fetch_sub(1, std::sync::atomic::Ordering::Release) - 1;
                    (ref_count, state.vk_obj)
                };

                self.by_load_handle.remove(&load_handle);

                if ref_count == 0 {
                    self.values.remove(&hash);
                    self.by_hash.remove(&hash);

                    // Return the underlying object if it is ready to be destroyed
                    Some(vk_obj)
                } else {
                    None
                }
            },
            None => {
                log::error!("A load handle was removed from a PipelineResourceManager but the passed in load handle was not found.");
                None
            }
        }
    }

    // Intended for use when destroying so that resources can be cleaned up
    fn take_all_resources(&mut self) -> Vec<GpuResourceT> {
        let resources = self.by_hash.iter().map(|(_, v)| v.vk_obj).collect();
        self.by_hash.clear();
        self.values.clear();
        self.by_load_handle.clear();
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
    load_handle: LoadHandleVersion,
    load_op: AssetLoadOp,
    description: T,
}

pub struct CommitRequest<T> {
    load_handle: LoadHandleVersion,
    phantom_data: PhantomData<T>
}

pub struct FreeRequest<T> {
    load_handle: LoadHandleVersion,
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
    graphics_pipelines: PipelineResourceManager<dsc::GraphicsPipeline, vk::Pipeline>,

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
        load_handle: LoadHandleVersion,
        shader_module: &dsc::ShaderModule,
    ) -> VkResult<vk::ShaderModule> {
        let hash = hash_resource_description(&shader_module);
        if self.shader_modules.contains_resource(hash) {
            Ok(self.shader_modules.add_ref(hash, load_handle, shader_module))
        } else {
            let resource =
                crate::pipeline_description::create_shader_module(self.device_context.device(), shader_module)?;
            self.shader_modules.insert(hash, load_handle, shader_module, resource);
            Ok(resource)
        }
    }

    pub fn load_descriptor_set_layout(
        &mut self,
        load_handle: LoadHandleVersion,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<vk::DescriptorSetLayout> {
        let hash = hash_resource_description(&descriptor_set_layout);
        if self.descriptor_set_layouts.contains_resource(hash) {
            Ok(self.descriptor_set_layouts.add_ref(hash, load_handle, descriptor_set_layout))
        } else {
            let resource =
                crate::pipeline_description::create_descriptor_set_layout(self.device_context.device(), descriptor_set_layout)?;
            self.descriptor_set_layouts.insert(hash, load_handle, descriptor_set_layout, resource);
            Ok(resource)
        }
    }

    pub fn load_pipeline_layout(
        &mut self,
        load_handle: LoadHandleVersion,
        pipeline_layout: &dsc::PipelineLayout
    ) -> VkResult<vk::PipelineLayout> {

        let hash = hash_resource_description(&pipeline_layout);
        if self.pipeline_layouts.contains_resource(hash) {
            Ok(self.pipeline_layouts.add_ref(hash, load_handle, pipeline_layout))
        } else {
            let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout.descriptor_set_layouts.len());
            for descriptor_set_layout in &pipeline_layout.descriptor_set_layouts {
                descriptor_set_layouts.push(self.load_descriptor_set_layout(load_handle, descriptor_set_layout)?);
            }

            let resource =
                crate::pipeline_description::create_pipeline_layout(self.device_context.device(), pipeline_layout, &descriptor_set_layouts)?;
            self.pipeline_layouts.insert(hash, load_handle, pipeline_layout, resource);
            Ok(resource)
        }
    }

    pub fn load_renderpass(
        &mut self,
        load_handle: LoadHandleVersion,
        renderpass: &dsc::RenderPass,
    ) -> VkResult<vk::RenderPass> {
        let hash = hash_resource_description(&renderpass);
        if self.renderpasses.contains_resource(hash) {
            Ok(self.renderpasses.add_ref(hash, load_handle, renderpass))
        } else {
            let resource =
                crate::pipeline_description::create_renderpass(self.device_context.device(), renderpass, self.swapchain_surface_info.as_ref().unwrap())?;
            self.renderpasses.insert(hash, load_handle, renderpass, resource);
            Ok(resource)
        }
    }

    pub fn load_graphics_pipeline(
        &mut self,
        load_handle: LoadHandleVersion,
        graphics_pipeline: &dsc::GraphicsPipeline,
    ) -> VkResult<vk::Pipeline> {
        let hash = hash_resource_description(&graphics_pipeline);
        if self.graphics_pipelines.contains_resource(hash) {
            Ok(self.graphics_pipelines.add_ref(hash, load_handle, graphics_pipeline))
        } else {
            let pipeline_layout = self.load_pipeline_layout(load_handle, &graphics_pipeline.pipeline_layout)?;
            let renderpass = self.load_renderpass(load_handle, &graphics_pipeline.renderpass)?;

            let mut shader_modules = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.stages.len());
            for stage in &graphics_pipeline.pipeline_shader_stages.stages {
                let shader_module = self.load_shader_module(load_handle, &stage.shader_module)?;
                shader_modules.push(shader_module);
            }

            let resource =
                crate::pipeline_description::create_graphics_pipeline(
                    self.device_context.device(),
                    graphics_pipeline,
                    pipeline_layout,
                    renderpass,
                    &shader_modules,
                    self.swapchain_surface_info.as_ref().unwrap()
                )?;
            self.graphics_pipelines.insert(hash, load_handle, graphics_pipeline, resource);
            Ok(resource)
        }
    }

    pub fn update(&mut self) {
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
                let shaders = Vec::with_capacity(request.description.pipeline_shader_stages.len());
                for shader_stage in request.description.pipeline_shader_stages {
                    //TODO: Get the thing by load handle
                    // - not sure if the load handle will be the correct version
                    //shader_stage.shader_module.load_handle()
                    self.shader_modules.get();
                }


                let result = self.load_graphics_pipeline(request.load_handle, &request.description);
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
        let load_handle_version = LoadHandleVersion {
            load_handle,
            version
        };

        let description = dsc::ShaderModule {
            code: asset.data.clone()
        };

        let request = LoadRequest {
            load_handle: load_handle_version,
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
        let load_handle_version = LoadHandleVersion {
            load_handle,
            version
        };

        let request = CommitRequest {
            load_handle: load_handle_version,
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
        let load_handle_version = LoadHandleVersion {
            load_handle,
            version
        };

        let request = FreeRequest {
            load_handle: load_handle_version,
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
        let load_handle_version = LoadHandleVersion {
            load_handle,
            version
        };

        let description = asset.clone();

        let request = LoadRequest {
            load_handle: load_handle_version,
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
        let load_handle_version = LoadHandleVersion {
            load_handle,
            version
        };

        let request = CommitRequest {
            load_handle: load_handle_version,
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
        let load_handle_version = LoadHandleVersion {
            load_handle,
            version
        };

        let request = FreeRequest {
            load_handle: load_handle_version,
            phantom_data: Default::default()
        };

        self.load_queues.free_request_tx.send(request);
    }
}