
use std::sync::Arc;
use crossbeam_channel::{Sender, Receiver};
use atelier_assets::loader::LoadHandle;
use fnv::FnvHashMap;
use ash::vk;
use crate::pipeline_description as dsc;
use renderer_shell_vulkan::{VkDropSinkResourceImpl, VkResourceDropSink, VkDeviceContext};
use std::hash::Hash;
use std::marker::PhantomData;
use atelier_assets::loader::handle::AssetHandle;

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct ResourceHash(u64);

impl ResourceHash {
    pub fn from_key<KeyT : Hash>(key: &KeyT) -> ResourceHash {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        ResourceHash(hasher.finish())
    }
}




//
// A reference counted object that sends a signal when it's dropped
//
struct ResourceArcInner<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    resource: ResourceT,
    drop_tx: Sender<ResourceT>
}

impl<ResourceT> Drop for ResourceArcInner<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    fn drop(&mut self) {
        self.drop_tx.send(self.resource);
    }
}

#[derive(Clone)]
struct ResourceArc<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    inner: Arc<ResourceArcInner<ResourceT>>
}

impl<ResourceT> ResourceArc<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    fn new(resource: ResourceT, drop_tx: Sender<ResourceT>) -> Self {
        ResourceArc {
            inner: Arc::new(ResourceArcInner {
                resource,
                drop_tx
            })
        }
    }

    fn get_raw(&self) -> ResourceT {
        *self.inner.resource.borrow()
    }
}

//
// A lookup of resources. They reference count using Arcs internally and send a signal when they
// drop. This allows the resources to be collected and disposed of
//
struct ResourceLookup<KeyT, ResourceT>
    where
        KeyT: Eq + Hash + Clone,
        ResourceT: VkDropSinkResourceImpl + Copy
{
    resources: FnvHashMap<ResourceHash, ResourceArc<ResourceT>>,
    keys: FnvHashMap<ResourceHash, KeyT>,
    drop_sink: VkResourceDropSink<ResourceT>,
    drop_tx: Sender<ResourceT>,
    drop_rx: Receiver<ResourceT>,
}

impl<KeyT, ResourceT> ResourceLookup<KeyT, ResourceT>
    where
        KeyT: Eq + Hash + Clone,
        ResourceT: VkDropSinkResourceImpl + Copy
{
    fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        ResourceLookup {
            resources: Default::default(),
            keys: Default::default(),
            drop_sink: VkResourceDropSink::new(max_frames_in_flight),
            drop_tx,
            drop_rx
        }
    }

    fn get(&self, hash: ResourceHash, key: &KeyT) -> Option<ResourceArc<ResourceT>> {
        self.resources.get(&hash).map(|x| {
            assert!(self.keys.get(&hash).unwrap() == key);
            x.clone()
        })

    }

    fn insert(&mut self, hash: ResourceHash, key: &KeyT, resource: ResourceT) -> ResourceArc<ResourceT> {
        let arc = ResourceArc::new(resource, self.drop_tx.clone());
        let old = self.resources.insert(hash, arc.clone());
        assert!(old.is_none());
        let old = self.keys.insert(hash, key.clone());
        assert!(old.is_none());
        arc
    }

    fn update(&mut self, device: &ash::Device) {
        for dropped in self.drop_rx.try_iter() {
            self.drop_sink.retire(dropped);
        }

        self.drop_sink.on_frame_complete(device);
    }

    fn len(&self) -> usize {
        self.resources.len()
    }
}

//
// Keys for each resource type. (Some keys are simple and use types from crate::pipeline_description
// and some are a combination of the definitions and runtime state. (For example, combining a
// renderpass with the swapchain surface it would be applied to)
//
#[derive(Clone, PartialEq, Eq, Hash)]
struct RenderPassKey {
    dsc: dsc::RenderPass,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct GraphicsPipelineKey {
    pipeline_layout: dsc::PipelineLayout,
    renderpass: dsc::RenderPass,
    fixed_function_state: dsc::FixedFunctionState,
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_load_handles: Vec<LoadHandle>,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo
}

//
// Handles raw lookup and destruction of GPU resources. Everything is reference counted. No safety
// is provided for dependencies/order of destruction. The general expectation is that anything
// dropped can safely be destroyed after a few frames have passed (based on max number of frames
// that can be submitted to the GPU)
//
struct ResourceLookupSet {
    pub shader_modules: ResourceLookup<dsc::ShaderModule, vk::ShaderModule>,
    pub descriptor_set_layouts: ResourceLookup<dsc::DescriptorSetLayout, vk::DescriptorSetLayout>,
    pub pipeline_layouts: ResourceLookup<dsc::PipelineLayout, vk::PipelineLayout>,
    pub render_passes: ResourceLookup<RenderPassKey, vk::RenderPass>,
    pub graphics_pipelines: ResourceLookup<GraphicsPipelineKey, vk::Pipeline>,
}

impl ResourceLookupSet {
    pub fn new(max_frames_in_flight: u32) -> Self {
        ResourceLookupSet {
            shader_modules: ResourceLookup::new(max_frames_in_flight),
            descriptor_set_layouts: ResourceLookup::new(max_frames_in_flight),
            pipeline_layouts: ResourceLookup::new(max_frames_in_flight),
            render_passes: ResourceLookup::new(max_frames_in_flight),
            graphics_pipelines: ResourceLookup::new(max_frames_in_flight),
        }
    }
}







//
// The "loaded" state of assets. Assets may have dependencies. Arcs to those dependencies ensure
// they do not get destroyed. All of the raw resources are hashed to avoid duplicating anything that
// is functionally identical. So for example if you have two windows with identical swapchain
// surfaces, they could share the same renderpass/pipeline resources
//
struct LoadedShaderModule {
    shader_module: ResourceArc<vk::ShaderModule>
}

struct LoadedDescriptorSetLayout {
    descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>
}

struct LoadedPipelineLayout {
    pipeline_layout: ResourceArc<vk::PipelineLayout>,
    //descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>
}

struct LoadedRenderPass {
    renderpass: ResourceArc<vk::RenderPass>,
}


struct CreatedGraphicsPipeline {
    //shader_modules: Vec<ResourceArc<vk::ShaderModule>>,
    //descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>,
    //pipeline_layout: ResourceArc<vk::PipelineLayout>,

    // Potentially one of these per swapchain surface
    renderpass: ResourceArc<vk::RenderPass>,
    pipeline: ResourceArc<vk::Pipeline>
}

struct LoadedGraphicsPipeline {
    shader_modules: Vec<ResourceArc<vk::ShaderModule>>,
    descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>,
    pipeline_layout: ResourceArc<vk::PipelineLayout>,

    // Potentially one of these per swapchain surface
    render_passes: Vec<ResourceArc<vk::RenderPass>>,
    pipelines: Vec<ResourceArc<vk::Pipeline>>
}


// struct LoadedShaderModule {
//     shader_module: CreatedShaderModule
// }
//
// struct LoadedGraphicsPipeline {
//     pipelines: Vec<CreatedGraphicsPipeline>
// }

//
// Represents a single asset which may simultaneously have committed and uncommitted loaded state
//
struct LoadedAssetState<LoadedAssetT> {
    committed: Option<LoadedAssetT>,
    uncommitted: Option<LoadedAssetT>
}

impl<LoadedAssetT> Default for LoadedAssetState<LoadedAssetT> {
    fn default() -> Self {
        LoadedAssetState {
            committed: None,
            uncommitted: None,
        }
    }
}

struct AssetLookup<LoadedAssetT> {
    loaded_assets: FnvHashMap<LoadHandle, LoadedAssetState<LoadedAssetT>>
}

impl<LoadedAssetT> AssetLookup<LoadedAssetT> {
    fn set_uncommitted(&mut self, load_handle: LoadHandle, loaded_asset: LoadedAssetT) {
        self.loaded_assets.entry(load_handle)
            .or_default()
            .uncommitted = Some(loaded_asset);
    }

    fn commit(&mut self, load_handle: LoadHandle) {
        let state = self.loaded_assets.get_mut(&load_handle).unwrap();
        state.committed = state.uncommitted.take();
    }

    fn free(&mut self, load_handle: LoadHandle) {
        let old = self.loaded_assets.remove(&load_handle);
        assert!(old.is_some());
    }

    fn get_latest(&mut self, load_handle: LoadHandle) -> Option<&LoadedAssetT> {
        if let Some(loaded_assets) = self.loaded_assets.get(&load_handle) {
            if let Some(uncommitted) = &loaded_assets.uncommitted {
                Some(uncommitted)
            } else if let Some(committed) = &loaded_assets.committed {
                Some(committed)
            } else {
                // It's an error to reach here because of uncommitted and committed are none, there
                // shouldn't be an entry in loaded_assets
                unreachable!();
                None
            }
        } else {
            None
        }
    }

    fn get_committed(&mut self, load_handle: LoadHandle) -> Option<&LoadedAssetT> {
        if let Some(loaded_assets) = self.loaded_assets.get(&load_handle) {
            if let Some(committed) = &loaded_assets.committed {
                Some(committed)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<LoadedAssetT> Default for AssetLookup<LoadedAssetT> {
    fn default() -> Self {
        AssetLookup {
            loaded_assets: Default::default()
        }
    }
}

//
// Lookups by asset for loaded asset state
//
#[derive(Default)]
struct LoadedAssetLookupSet {
    pub shader_modules: AssetLookup<LoadedShaderModule>,
    pub graphics_pipelines: AssetLookup<LoadedGraphicsPipeline>,
}


















use atelier_assets::loader::AssetLoadOp;
use atelier_assets::core::AssetUuid;





//
// Message handling for asset load/commit/free events
//
pub struct LoadRequest<T> {
    load_handle: LoadHandle,
    load_op: AssetLoadOp,
    asset: T,
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


//
// A generic load handler that allows routing load/commit/free events
//
use crate::asset_storage::ResourceHandle;
use crate::asset_storage::ResourceLoadHandler;
use type_uuid::TypeUuid;
use crate::pipeline::pipeline::PipelineAsset;
use crate::pipeline::shader::ShaderAsset;
use ash::prelude::VkResult;
use crate::pipeline_description::SwapchainSurfaceInfo;
use std::hint::unreachable_unchecked;
use std::borrow::Borrow;

pub struct GenericLoadHandler<AssetT>
    where
        AssetT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
{
    load_queues: LoadQueuesTx<AssetT>
}

impl<AssetT> ResourceLoadHandler<AssetT> for GenericLoadHandler<AssetT>
    where
        AssetT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
{
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<AssetT>,
        version: u32,
        asset: &AssetT,
        load_op: AssetLoadOp
    ) {
        let request = LoadRequest {
            load_handle,
            load_op,
            asset: asset.clone()
        };

        self.load_queues.load_request_tx.send(request);
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<AssetT>,
        version: u32,
        asset: &AssetT
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
        resource_handle: ResourceHandle<AssetT>,
        version: u32,
    ) {
        let request = FreeRequest {
            load_handle,
            phantom_data: Default::default()
        };

        self.load_queues.free_request_tx.send(request);
    }
}

#[derive(Default)]
struct LoadQueueSet {
    shader_modules: LoadQueues<ShaderAsset>,
    graphics_pipelines: LoadQueues<PipelineAsset>
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

    pub fn unique_swapchain_infos(&self) -> &Vec<dsc::SwapchainSurfaceInfo> {
        &self.unique_swapchain_infos
    }
}






















struct ResourceManager {
    device_context: VkDeviceContext,

    resources: ResourceLookupSet,
    loaded_assets: LoadedAssetLookupSet,
    load_queues: LoadQueueSet,
    swapchain_surfaces: ActiveSwapchainSurfaceInfoSet,

}

impl ResourceManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        ResourceManager {
            device_context: device_context.clone(),
            resources: ResourceLookupSet::new(renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32),
            loaded_assets: Default::default(),
            load_queues: Default::default(),
            swapchain_surfaces: Default::default(),
        }
    }

    pub fn create_shader_load_handler(&self) -> GenericLoadHandler<ShaderAsset> {
        GenericLoadHandler {
            load_queues: self.load_queues.shader_modules.tx.clone()
        }
    }

    pub fn create_pipeline_load_handler(&self) -> GenericLoadHandler<PipelineAsset> {
        GenericLoadHandler {
            load_queues: self.load_queues.graphics_pipelines.tx.clone()
        }
    }

    pub fn add_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo
    ) -> VkResult<()> {
        unimplemented!();
    }

    pub fn remove_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo
    ) {
        unimplemented!();
    }

    pub fn update(
        &mut self,
    ) {
        self.process_shader_load_requests();
        self.process_pipeline_load_requests();
        self.dump_stats();
    }

    fn dump_stats(&self) {
        #[derive(Debug)]
        struct ResourceCounts {
            shader_module_count: usize,
            descriptor_set_layout_count: usize,
            pipeline_layout_count: usize,
            renderpass_count: usize,
            pipeline_count: usize,
        }

        let shader_module_count = self.resources.shader_modules.len();
        let descriptor_set_layout_count = self.resources.descriptor_set_layouts.len();
        let pipeline_layout_count = self.resources.pipeline_layouts.len();
        let renderpass_count = self.resources.render_passes.len();
        let pipeline_count = self.resources.graphics_pipelines.len();

        let resource_counts = ResourceCounts {
            shader_module_count,
            descriptor_set_layout_count,
            pipeline_layout_count,
            renderpass_count,
            pipeline_count,
        };
    }

    fn process_shader_load_requests(&mut self) {
        for request in self.load_queues.shader_modules.take_load_requests() {
            let shader_module = dsc::ShaderModule {
                code: request.asset.data
            };

            let loaded_asset = self.load_shader_module(
                &shader_module
            );

            match loaded_asset {
                Ok(loaded_asset) => {
                    self.loaded_assets.shader_modules.set_uncommitted(request.load_handle, loaded_asset);
                    request.load_op.complete()
                },
                Err(err) => {
                    request.load_op.error(err);
                }
            }
        }

        for request in self.load_queues.shader_modules.take_commit_requests() {
            self.loaded_assets.shader_modules.commit(request.load_handle);
        }

        for request in self.load_queues.shader_modules.take_free_requests() {
            self.loaded_assets.shader_modules.free(request.load_handle);
        }
    }

    fn process_pipeline_load_requests(&mut self) {
        for request in self.load_queues.graphics_pipelines.take_load_requests() {
            let loaded_asset = self.load_graphics_pipeline(
                &request.asset
            );

            match loaded_asset {
                Ok(loaded_asset) => {
                    self.loaded_assets.graphics_pipelines.set_uncommitted(request.load_handle, loaded_asset);
                    request.load_op.complete()
                },
                Err(err) => {
                    //TODO: May need to unregister upstream dependencies (like shaders, pipeline layouts, descriptor sets)
                    request.load_op.error(err);
                }
            }
        }

        for request in self.load_queues.graphics_pipelines.take_commit_requests() {
            self.loaded_assets.graphics_pipelines.commit(request.load_handle);
        }

        for request in self.load_queues.graphics_pipelines.take_free_requests() {
            self.loaded_assets.graphics_pipelines.free(request.load_handle);
        }
    }









    fn load_shader_module(
        &mut self,
        //load_handle: LoadHandle,
        shader_module: &dsc::ShaderModule,
    ) -> VkResult<LoadedShaderModule> {
        let hash = ResourceHash::from_key(shader_module);
        if let Some(shader_module) = self.resources.shader_modules.get(hash, shader_module) {
            Ok(LoadedShaderModule {
                shader_module
            })
        } else {
            println!("Creating shader module\n[bytes: {}]", shader_module.code.len());
            let resource =
                crate::pipeline_description::create_shader_module(self.device_context.device(), shader_module)?;
            let shader_module = self.resources.shader_modules.insert(hash, shader_module, resource);
            Ok(LoadedShaderModule {
                shader_module
            })
        }
    }




    fn load_descriptor_set_layout(
        &mut self,
        //load_handle: LoadHandle,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<LoadedDescriptorSetLayout> {
        let hash = ResourceHash::from_key(descriptor_set_layout);
        if let Some(descriptor_set_layout) = self.resources.descriptor_set_layouts.get(hash, descriptor_set_layout) {
            Ok(LoadedDescriptorSetLayout {
                descriptor_set_layout
            })
        } else {
            println!("Creating descriptor set layout\n{:#?}", descriptor_set_layout);
            let resource =
                crate::pipeline_description::create_descriptor_set_layout(self.device_context.device(), descriptor_set_layout)?;
            let descriptor_set_layout = self.resources.descriptor_set_layouts.insert(hash, descriptor_set_layout, resource);
            Ok(LoadedDescriptorSetLayout {
                descriptor_set_layout
            })
        }
    }

    fn load_pipeline_layout(
        &mut self,
        //load_handle: LoadHandle,
        pipeline_layout_def: &dsc::PipelineLayout
    ) -> VkResult<LoadedPipelineLayout> {
        let hash = ResourceHash::from_key(pipeline_layout_def);
        if let Some(pipeline_layout) = self.resources.pipeline_layouts.get(hash, pipeline_layout_def) {
            Ok(LoadedPipelineLayout {
                pipeline_layout,
                //descriptor_set_layouts
            })
        } else {
            // Keep both the arcs and build an array of vk object pointers
            let mut descriptor_set_layout_arcs = Vec::with_capacity(pipeline_layout_def.descriptor_set_layouts.len());
            let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout_def.descriptor_set_layouts.len());

            for descriptor_set_layout_def in &pipeline_layout_def.descriptor_set_layouts {
                let loaded_descriptor_set_layout = self.load_descriptor_set_layout(descriptor_set_layout_def)?;
                //for descriptor_set_layout_arc in loaded_descriptor_set_layout.descriptor_set_layout {
                    descriptor_set_layout_arcs.push(loaded_descriptor_set_layout.descriptor_set_layout.clone());
                    descriptor_set_layouts.push(loaded_descriptor_set_layout.descriptor_set_layout.get_raw());
                //}
            }

            println!("Creating pipeline layout\n{:#?}", pipeline_layout_def);
            let resource =
                crate::pipeline_description::create_pipeline_layout(self.device_context.device(), pipeline_layout_def, &descriptor_set_layouts)?;
            let pipeline_layout = self.resources.pipeline_layouts.insert(hash, pipeline_layout_def, resource);

            Ok(LoadedPipelineLayout {
                pipeline_layout,
                //descriptor_set_layout_arcs,
            })
        }
    }

    // Shared logic pulled out because it may be called when loading an asset or
    // fn create_renderpass(
    //     device_context: &VkDeviceContext,
    //     swapchain_infos: &SwapchainSurfaceInfo,
    //     renderpass: &dsc::RenderPass,
    // ) -> VkResult<vk::RenderPass> {
    //     println!("Creating renderpasses\n{:#?}", renderpass);
    //     let mut resources = Vec::with_capacity(swapchain_infos.len());
    //     let resource =
    //         crate::pipeline_description::create_renderpass(device_context.device(), renderpass, swapchain_info)?;
    //     resources.push(resource);
    //
    //     Ok(resources)
    // }

    fn load_renderpass(
        &mut self,
        //load_handle: LoadHandle,
        renderpass: &dsc::RenderPass,
        swapchain_surface_info: &SwapchainSurfaceInfo
    ) -> VkResult<LoadedRenderPass> {
        // let swapchain_surface_infos = self.active_swapchain_surface_infos.unique_swapchain_infos();
        // let mut render_passes = Vec::with_capacity(swapchain_surface_infos.len());
        //
        // for swapchain_surface_info in swapchain_surface_infos {
            let renderpass_key = RenderPassKey {
                dsc: renderpass.clone(),
                swapchain_surface_info: swapchain_surface_info.clone()
            };

            let hash = ResourceHash::from_key(&renderpass_key);
            if let Some(renderpass) = self.resources.render_passes.get(hash, &renderpass_key) {
                // render_passes.push(LoadedRenderPass {
                //     renderpass
                // });
                Ok(LoadedRenderPass {
                    renderpass
                })
            } else {
                let resource = crate::pipeline_description::create_renderpass(
                    self.device_context.device(),
                    renderpass,
                    &swapchain_surface_info,
                )?;

                let renderpass = self.resources.render_passes.insert(hash, &renderpass_key, resource);
                //render_passes.push(renderpass);

                Ok(LoadedRenderPass {
                    renderpass
                })
            }
        // }
        //
        // Ok(LoadedRenderPass {
        //     render_passes
        // })
    }

    fn create_graphics_pipelines(
        device_context: &VkDeviceContext,
        shader_modules: &[vk::ShaderModule],
        pipeline_layout: vk::PipelineLayout,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
        renderpass: vk::RenderPass,
        graphics_pipeline: &PipelineAsset,
    ) -> VkResult<vk::Pipeline> {
        let mut shader_modules_meta = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
        for stage in &graphics_pipeline.pipeline_shader_stages {
            let shader_module_meta = dsc::ShaderModuleMeta {
                stage: stage.stage,
                entry_name: stage.entry_name.clone()
            };
            shader_modules_meta.push(shader_module_meta);
        }

        println!("Creating graphics pipeline\n{:#?}\n{:#?}", graphics_pipeline.fixed_function_state, shader_modules_meta);

        //let mut resources = Vec::with_capacity(swapchain_infos.len());
        //for (swapchain_surface_info, renderpass) in swapchain_infos.iter().zip(renderpasses) {
            let resource =
                crate::pipeline_description::create_graphics_pipeline(
                    device_context.device(),
                    &graphics_pipeline.fixed_function_state,
                    pipeline_layout,
                    renderpass,
                    &shader_modules_meta,
                    &shader_modules,
                    swapchain_surface_info
                )?;
                Ok(resource)


            //resources.push(resource);
        //}

        //Ok(resources)
    }

    fn load_graphics_pipeline(
        &mut self,
        //load_handle: LoadHandle,
        graphics_pipeline: &PipelineAsset,
    ) -> VkResult<LoadedGraphicsPipeline> {
        //TODO: Hashing the asset comes with the downside that if shader assets are different load
        // handles but the same values, we don't deduplicate them.

        //
        // Shader module metadata (required to create the pipeline key)
        //
        let mut shader_module_metas = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
        let mut shader_module_load_handles = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
        for stage in &graphics_pipeline.pipeline_shader_stages {
            let shader_module_meta = dsc::ShaderModuleMeta {
                stage: stage.stage,
                entry_name: stage.entry_name.clone()
            };
            shader_module_metas.push(shader_module_meta);
            shader_module_load_handles.push(stage.shader_module.load_handle());
        }

        //
        // Actual shader module resources (to create the pipeline)
        //
        let mut shader_module_arcs = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
        let mut shader_module_vk_objs = Vec::with_capacity(graphics_pipeline.pipeline_shader_stages.len());
        for stage in &graphics_pipeline.pipeline_shader_stages {
            let shader_module = self.loaded_assets.shader_modules.get_latest(stage.shader_module.load_handle()).unwrap();
            shader_module_arcs.push(shader_module.shader_module.clone());
            shader_module_vk_objs.push(shader_module.shader_module.get_raw());
        }

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layouts = Vec::with_capacity(graphics_pipeline.pipeline_layout.descriptor_set_layouts.len());
        for descriptor_set_layout_def in &graphics_pipeline.pipeline_layout.descriptor_set_layouts {
            let descriptor_set_layout = self.load_descriptor_set_layout(descriptor_set_layout_def)?;
            descriptor_set_layouts.push(descriptor_set_layout.descriptor_set_layout);
        }

        //
        // Pipeline layout
        //
        let pipeline_layout = self.load_pipeline_layout(&graphics_pipeline.pipeline_layout)?;

        //
        // Render passes
        //
        let swapchain_surface_infos = self.swapchain_surfaces.unique_swapchain_infos().clone();
        let mut render_passes = Vec::with_capacity(self.swapchain_surfaces.unique_swapchain_infos().len());
        for swapchain_surface_info in &swapchain_surface_infos {
            let render_pass = self.load_renderpass(&graphics_pipeline.renderpass, swapchain_surface_info)?;
            render_passes.push(render_pass.renderpass);
        }


        //
        // Render passes and pipelines
        //
        let mut pipelines = Vec::with_capacity(self.swapchain_surfaces.unique_swapchain_infos().len());

        for (swapchain_surface_info, renderpass) in swapchain_surface_infos.iter().zip(&render_passes) {
            let pipeline_key = GraphicsPipelineKey {
                shader_module_load_handles: shader_module_load_handles.clone(),
                shader_module_metas: shader_module_metas.clone(),
                fixed_function_state: graphics_pipeline.fixed_function_state.clone(),
                pipeline_layout: graphics_pipeline.pipeline_layout.clone(),
                renderpass: graphics_pipeline.renderpass.clone(),
                swapchain_surface_info: swapchain_surface_info.clone()
            };

            let hash = ResourceHash::from_key(&pipeline_key);
            if let Some(resource) = self.resources.graphics_pipelines.get(hash, &pipeline_key) {
                pipelines.push(resource);
            } else {
                // let resource = Self::create_graphics_pipelines(
                //     &self.device_context,
                //     &shader_module_vk_objs,
                //     pipeline_layout.pipeline_layout.get_raw(),
                //     &swapchain_surface_info,
                //     renderpass.get_raw(),
                //     graphics_pipeline
                // )?;

                let resource = crate::pipeline_description::create_graphics_pipeline(
                    &self.device_context.device(),
                    &graphics_pipeline.fixed_function_state,
                    pipeline_layout.pipeline_layout.get_raw(),
                    renderpass.get_raw(),
                    &shader_module_metas,
                    &shader_module_vk_objs,
                    swapchain_surface_info
                )?;

                let pipeline = self.resources.graphics_pipelines.insert(hash, &pipeline_key, resource);

                let graphics_pipeline = CreatedGraphicsPipeline {
                    //shader_modules: shader_module_arcs.clone(),
                    //descriptor_set_layouts: descriptor_set_layouts.clone(),
                    //pipeline_layout: pipeline_layout.pipeline_layout.clone(),
                    renderpass: renderpass.clone(),
                    pipeline: pipeline.clone(),
                };

                pipelines.push(pipeline);


                // self.graphics_pipelines.insert(load_handle, hash, graphics_pipeline, hashed_graphics_pipeline.clone());
                // Ok(hashed_graphics_pipeline)
            }
        }

        Ok(LoadedGraphicsPipeline {
            shader_modules: shader_module_arcs,
            descriptor_set_layouts,
            pipeline_layout: pipeline_layout.pipeline_layout,
            render_passes,
            pipelines
        })
    }
}
















/*

struct AssetState<T> {
    committed: SharedResource<T>,
    uncommitted: SharedResource<T>
}

struct LoadedShaderModule {
    vk_obj: vk::ShaderModule,
}

struct LoadedPipeline {
    vk_obj: vk::ShaderModule,
}

struct ShaderModuleKey {
    description: dsc::ShaderModule
}

struct PipelineKey {
    description: dsc::ShaderModule,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}

struct AssetLookup {
    // Every single unique thing
    all_shader_modules: FnvHashMap<HaderModuleKey, SharedResource<vk::ShaderModule>>,
    all_pipelines: FnvHashMap<PipelineKey, SharedResource<vk::Pipeline>>,

    //
    committed_shader_modules: FnvHashMap<LoadHandle, >
}

impl AssetLookup {
    //
    // Swapchain API
    //
    pub fn add_swapchain() {

    }

    pub fn remove_swapchain() {

    }

    //
    // Asset loading API
    //
    pub fn load(load_handle: LoadHandle) {

    }

    pub fn commit(load_handle: LoadHandle) {

    }

    pub fn free(load_handle: LoadHandle) {

    }

    //
    // Asset lookup API
    //
    pub fn get_committed(load_handle: LoadHandle) {

    }

    pub fn get_latest(load_handle: LoadHandle) {

    }
}

*/