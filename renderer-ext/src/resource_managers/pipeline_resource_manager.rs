
use std::sync::Arc;
use std::sync::Weak;
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
#[derive(Clone)]
struct ResourceWithHash<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    resource: ResourceT,
    resource_hash: ResourceHash,
}

struct ResourceArcInner<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    resource: ResourceWithHash<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>
}

impl<ResourceT> Drop for ResourceArcInner<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    fn drop(&mut self) {
        self.drop_tx.send(self.resource.clone());
    }
}

#[derive(Clone)]
pub struct WeakResourceArc<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    inner: Weak<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> WeakResourceArc<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    pub fn upgrade(&self) -> Option<ResourceArc<ResourceT>> {
        if let Some(upgrade) = self.inner.upgrade() {
            Some(ResourceArc {
                inner: upgrade
            })
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct ResourceArc<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    inner: Arc<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> ResourceArc<ResourceT>
    where
        ResourceT: VkDropSinkResourceImpl + Copy
{
    fn new(
        resource: ResourceT,
        resource_hash: ResourceHash,
        drop_tx: Sender<ResourceWithHash<ResourceT>>
    ) -> Self {
        ResourceArc {
            inner: Arc::new(ResourceArcInner {
                resource: ResourceWithHash {
                    resource,
                    resource_hash
                },
                drop_tx
            })
        }
    }

    pub fn get_raw(&self) -> ResourceT {
        self.inner.resource.borrow().resource
    }

    pub fn downgrade(&self) -> WeakResourceArc<ResourceT> {
        let inner = Arc::downgrade(&self.inner);
        WeakResourceArc {
            inner
        }
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
    resources: FnvHashMap<ResourceHash, WeakResourceArc<ResourceT>>,
    keys: FnvHashMap<ResourceHash, KeyT>,
    drop_sink: VkResourceDropSink<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    drop_rx: Receiver<ResourceWithHash<ResourceT>>,
}

impl<KeyT, ResourceT> ResourceLookup<KeyT, ResourceT>
    where
        KeyT: Eq + Hash + Clone,
        ResourceT: VkDropSinkResourceImpl + Copy + std::fmt::Debug
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
        if let Some(resource) = self.resources.get(&hash) {
            let upgrade = resource.upgrade();
            if upgrade.is_some() {
                assert!(self.keys.get(&hash).unwrap() == key);
            }

            upgrade
        } else {
            None
        }
    }

    fn insert(&mut self, hash: ResourceHash, key: &KeyT, resource: ResourceT) -> ResourceArc<ResourceT> {
        // Process any pending drops. If we don't do this, it's possible that the pending drop could
        // wipe out the state we're about to set
        self.handle_dropped_resources();

        println!("insert resource {} {:?}", core::any::type_name::<ResourceT>(), resource);

        let arc = ResourceArc::new(resource, hash, self.drop_tx.clone());
        let downgraded = arc.downgrade();
        let old = self.resources.insert(hash, downgraded);
        assert!(old.is_none());
        self.keys.insert(hash, key.clone());
        assert!(old.is_none());
        arc
    }

    fn handle_dropped_resources(&mut self) {
        for dropped in self.drop_rx.try_iter() {
            println!("dropping {} {:?}", core::any::type_name::<ResourceT>(), dropped.resource);
            self.drop_sink.retire(dropped.resource);
            self.resources.remove(&dropped.resource_hash);
            self.keys.remove(&dropped.resource_hash);
        }
    }

    fn len(&self) -> usize {
        self.resources.len()
    }

    fn on_frame_complete(&mut self, device: &ash::Device) {
        self.handle_dropped_resources();
        self.drop_sink.on_frame_complete(device);
    }

    fn destroy(&mut self, device: &ash::Device) {
        self.handle_dropped_resources();

        if self.resources.len() > 0 {
            log::warn!(
                "{} resource count {} > 0, resources will leak",
                core::any::type_name::<ResourceT>(),
                self.resources.len()
            );
        }

        self.drop_sink.destroy(device);
    }
}

//
// Keys for each resource type. (Some keys are simple and use types from crate::pipeline_description
// and some are a combination of the definitions and runtime state. (For example, combining a
// renderpass with the swapchain surface it would be applied to)
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RenderPassKey {
    dsc: dsc::RenderPass,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    fn on_frame_complete(&mut self, device: &ash::Device) {
        self.shader_modules.on_frame_complete(device);
        self.descriptor_set_layouts.on_frame_complete(device);
        self.pipeline_layouts.on_frame_complete(device);
        self.render_passes.on_frame_complete(device);
        self.graphics_pipelines.on_frame_complete(device);
    }

    fn destroy(&mut self, device: &ash::Device) {
        self.shader_modules.destroy(device);
        self.descriptor_set_layouts.destroy(device);
        self.pipeline_layouts.destroy(device);
        self.render_passes.destroy(device);
        self.graphics_pipelines.destroy(device);
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

struct LoadedGraphicsPipeline {
    shader_modules: Vec<ResourceArc<vk::ShaderModule>>,
    descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>,
    pipeline_layout: ResourceArc<vk::PipelineLayout>,

    // Potentially one of these per swapchain surface
    render_passes: Vec<ResourceArc<vk::RenderPass>>,
    pipelines: Vec<ResourceArc<vk::Pipeline>>,

    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pipeline_asset: PipelineAsset,
}






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

    fn get_committed(&self, load_handle: LoadHandle) -> Option<&LoadedAssetT> {
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

    fn len(&self) -> usize {
        self.loaded_assets.len()
    }

    fn destroy(&mut self) {
        // if self.loaded_assets.len() > 0 {
        //     log::warn!(
        //         "{} asset count {} > 0, resources will leak",
        //         core::any::type_name::<LoadedAssetT>(),
        //         self.loaded_assets.len(),
        //     );
        // }

        self.loaded_assets.clear();
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

impl LoadedAssetLookupSet {
    pub fn destroy(&mut self) {
        self.shader_modules.destroy();
        self.graphics_pipelines.destroy();
    }
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
use std::borrow::Borrow;
use atelier_assets::loader::handle::Handle;

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












struct DescriptorSetPoolManager {
    device_context: VkDeviceContext,
    descriptor_set_layout_def: dsc::DescriptorSetLayout,
    descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
    count_per_pool: u32,
    pools: Vec<vk::DescriptorPool>
}

impl DescriptorSetPoolManager {
    fn new(
        device_context: &VkDeviceContext,
        descriptor_set_layout_def: dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
        count_per_pool: u32,
    ) -> Self {
        DescriptorSetPoolManager {
            device_context: device_context.clone(),
            descriptor_set_layout_def,
            descriptor_set_layout,
            count_per_pool,
            pools: Default::default()
        }
    }

    fn allocate() {

    }
}


























pub struct PipelineInfo {
    pub descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>,
    pub pipeline_layout: ResourceArc<vk::PipelineLayout>,
    pub renderpass: ResourceArc<vk::RenderPass>,
    pub pipeline: ResourceArc<vk::Pipeline>,
}

pub struct ResourceManager {
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

    pub fn get_pipeline_info(&self, handle: &Handle<PipelineAsset>, swapchain: &SwapchainSurfaceInfo) -> PipelineInfo {
        let resource = self.loaded_assets.graphics_pipelines.get_committed(handle.load_handle()).unwrap();
        let swapchain_index = self.swapchain_surfaces.ref_counts.get(swapchain).unwrap().index;

        PipelineInfo {
            descriptor_set_layouts: resource.descriptor_set_layouts.clone(),
            pipeline_layout: resource.pipeline_layout.clone(),
            renderpass: resource.render_passes[swapchain_index].clone(),
            pipeline: resource.pipelines[swapchain_index].clone()
        }
    }

    pub fn add_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo
    ) -> VkResult<()> {
        log::info!("add_swapchain {:?}", swapchain_surface_info);
        // Add it
        if self.swapchain_surfaces.add(&swapchain_surface_info) {
            let mut committed_updates = vec![];
            let mut uncommitted_updates = vec![];

            // Allocate a new renderpass instance for every loaded renderpass for the swapchain
            for (load_handle, loaded_asset) in &mut self.loaded_assets.graphics_pipelines.loaded_assets {
                if let Some(committed) = &mut loaded_asset.committed {
                    committed_updates.push((load_handle.clone(), committed.pipeline_asset.clone()));
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    uncommitted_updates.push((load_handle.clone(), uncommitted.pipeline_asset.clone()));
                }
            }

            for (load_handle, pipeline_asset) in committed_updates {
                let renderpass = self.get_or_create_renderpass(
                    &pipeline_asset.renderpass,
                    swapchain_surface_info
                )?;

                let pipeline_create_data = PipelineCreateData::new(self, &pipeline_asset)?;
                let pipeline = self.get_or_create_pipeline(
                    &pipeline_create_data,
                    &pipeline_asset,
                    swapchain_surface_info,
                    renderpass.clone()
                )?;

                let mut loaded_graphics_pipeline = self.loaded_assets.graphics_pipelines.loaded_assets.get_mut(&load_handle).unwrap();
                let mut committed = loaded_graphics_pipeline.committed.as_mut().unwrap();
                committed.render_passes.push(renderpass);
                committed.pipelines.push(pipeline);
            }


            for (load_handle, pipeline_asset) in uncommitted_updates {
                let renderpass = self.get_or_create_renderpass(
                    &pipeline_asset.renderpass,
                    swapchain_surface_info
                )?;

                let pipeline_create_data = PipelineCreateData::new(self, &pipeline_asset)?;
                let pipeline = self.get_or_create_pipeline(
                    &pipeline_create_data,
                    &pipeline_asset,
                    swapchain_surface_info,
                    renderpass.clone()
                )?;

                let mut loaded_graphics_pipeline = self.loaded_assets.graphics_pipelines.loaded_assets.get_mut(&load_handle).unwrap();
                let mut uncommitted = loaded_graphics_pipeline.uncommitted.as_mut().unwrap();
                uncommitted.render_passes.push(renderpass);
                uncommitted.pipelines.push(pipeline);
            }
        }

        Ok(())
    }

    pub fn remove_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo
    ) {
        log::info!("remove_swapchain {:?}", swapchain_surface_info);
        //TODO: Common case is to destroy and re-create the same swapchain surface info, so we can
        // delay destroying until we also get an additional add/remove. If the next add call is
        // the same, we can avoid the remove entirely
        if let Some(remove_index) = self.swapchain_surfaces.remove(swapchain_surface_info) {
            for (_, loaded_asset) in &mut self.loaded_assets.graphics_pipelines.loaded_assets {
                if let Some(committed) = &mut loaded_asset.committed {
                    committed.render_passes.swap_remove(remove_index);
                    committed.pipelines.swap_remove(remove_index);
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    uncommitted.render_passes.swap_remove(remove_index);
                    uncommitted.pipelines.swap_remove(remove_index);
                }
            }
        } else {
            log::error!("Received a remove swapchain without a matching add\n{:#?}", swapchain_surface_info);
        }
    }

    pub fn update(
        &mut self,
    ) {
        self.process_shader_load_requests();
        self.process_pipeline_load_requests();
        //self.dump_stats();
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

        let resource_counts = ResourceCounts {
            shader_module_count: self.resources.shader_modules.len(),
            descriptor_set_layout_count: self.resources.descriptor_set_layouts.len(),
            pipeline_layout_count: self.resources.pipeline_layouts.len(),
            renderpass_count: self.resources.render_passes.len(),
            pipeline_count: self.resources.graphics_pipelines.len(),
        };

        #[derive(Debug)]
        struct LoadedAssetCounts {
            shader_module_count: usize,
            pipeline_count: usize,
        }

        let loaded_asset_counts = LoadedAssetCounts {
            shader_module_count: self.loaded_assets.shader_modules.len(),
            pipeline_count: self.loaded_assets.graphics_pipelines.len(),
        };

        #[derive(Debug)]
        struct ResourceManagerMetrics {
            resource_counts: ResourceCounts,
            loaded_asset_counts: LoadedAssetCounts
        }

        let metrics = ResourceManagerMetrics {
            resource_counts,
            loaded_asset_counts,
        };

        println!("Resource Manager Metrics:\n{:#?}", metrics);
    }

    fn process_shader_load_requests(&mut self) {
        for request in self.load_queues.shader_modules.take_load_requests() {
            let shader_module_def = dsc::ShaderModule {
                code: request.asset.data
            };

            println!("Create shader module {:?}", request.load_handle);
            let shader_module = self.get_or_create_shader_module(
                &shader_module_def
            );

            match shader_module {
                Ok(shader_module) => {
                    self.loaded_assets.shader_modules.set_uncommitted(request.load_handle, LoadedShaderModule {
                        shader_module
                    });
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
            println!("Create pipeline {:?}", request.load_handle);
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









    fn get_or_create_shader_module(
        &mut self,
        shader_module: &dsc::ShaderModule,
    ) -> VkResult<ResourceArc<vk::ShaderModule>> {
        let hash = ResourceHash::from_key(shader_module);
        if let Some(shader_module) = self.resources.shader_modules.get(hash, shader_module) {
            Ok(shader_module)
        } else {
            println!("Creating shader module\n[bytes: {}]", shader_module.code.len());
            let resource =
                crate::pipeline_description::create_shader_module(self.device_context.device(), shader_module)?;
            println!("Created shader module {:?}", resource);
            let shader_module = self.resources.shader_modules.insert(hash, shader_module, resource);
            Ok(shader_module)
        }
    }




    fn get_or_create_descriptor_set_layout(
        &mut self,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<ResourceArc<vk::DescriptorSetLayout>> {
        let hash = ResourceHash::from_key(descriptor_set_layout);
        if let Some(descriptor_set_layout) = self.resources.descriptor_set_layouts.get(hash, descriptor_set_layout) {
            Ok(descriptor_set_layout)
        } else {
            println!("Creating descriptor set layout\n{:#?}", descriptor_set_layout);
            let resource =
                crate::pipeline_description::create_descriptor_set_layout(self.device_context.device(), descriptor_set_layout)?;
            println!("Created descriptor set layout {:?}", resource);
            let descriptor_set_layout = self.resources.descriptor_set_layouts.insert(hash, descriptor_set_layout, resource);
            Ok(descriptor_set_layout)
        }
    }

    fn get_or_create_pipeline_layout(
        &mut self,
        pipeline_layout_def: &dsc::PipelineLayout
    ) -> VkResult<ResourceArc<vk::PipelineLayout>> {
        let hash = ResourceHash::from_key(pipeline_layout_def);
        if let Some(pipeline_layout) = self.resources.pipeline_layouts.get(hash, pipeline_layout_def) {
            Ok(pipeline_layout)
        } else {
            // Keep both the arcs and build an array of vk object pointers
            let mut descriptor_set_layout_arcs = Vec::with_capacity(pipeline_layout_def.descriptor_set_layouts.len());
            let mut descriptor_set_layouts = Vec::with_capacity(pipeline_layout_def.descriptor_set_layouts.len());

            for descriptor_set_layout_def in &pipeline_layout_def.descriptor_set_layouts {
                let loaded_descriptor_set_layout = self.get_or_create_descriptor_set_layout(descriptor_set_layout_def)?;
                descriptor_set_layout_arcs.push(loaded_descriptor_set_layout.clone());
                descriptor_set_layouts.push(loaded_descriptor_set_layout.get_raw());
            }

            println!("Creating pipeline layout\n{:#?}", pipeline_layout_def);
            let resource =
                crate::pipeline_description::create_pipeline_layout(self.device_context.device(), pipeline_layout_def, &descriptor_set_layouts)?;
            println!("Created pipeline layout {:?}", resource);
            let pipeline_layout = self.resources.pipeline_layouts.insert(hash, pipeline_layout_def, resource);

            Ok(pipeline_layout)
        }
    }

    fn get_or_create_renderpass(
        &mut self,
        renderpass: &dsc::RenderPass,
        swapchain_surface_info: &SwapchainSurfaceInfo
    ) -> VkResult<ResourceArc<vk::RenderPass>> {
        let renderpass_key = RenderPassKey {
            dsc: renderpass.clone(),
            swapchain_surface_info: swapchain_surface_info.clone()
        };

        let hash = ResourceHash::from_key(&renderpass_key);
        if let Some(renderpass) = self.resources.render_passes.get(hash, &renderpass_key) {
            Ok(renderpass)
        } else {
            let resource = crate::pipeline_description::create_renderpass(
                self.device_context.device(),
                renderpass,
                &swapchain_surface_info,
            )?;

            let renderpass = self.resources.render_passes.insert(hash, &renderpass_key, resource);
            Ok(renderpass)
        }
    }

    fn get_or_create_pipeline(
        &mut self,
        pipeline_create_data: &PipelineCreateData,
        pipeline_asset: &PipelineAsset,
        swapchain_surface_info: &SwapchainSurfaceInfo,
        renderpass: ResourceArc<vk::RenderPass>,
    ) -> VkResult<ResourceArc<vk::Pipeline>> {
        let pipeline_key = GraphicsPipelineKey {
            shader_module_load_handles: pipeline_create_data.shader_module_load_handles.clone(),
            shader_module_metas: pipeline_create_data.shader_module_metas.clone(),
            fixed_function_state: pipeline_asset.fixed_function_state.clone(),
            pipeline_layout: pipeline_asset.pipeline_layout.clone(),
            renderpass: pipeline_asset.renderpass.clone(),
            swapchain_surface_info: swapchain_surface_info.clone()
        };

        let hash = ResourceHash::from_key(&pipeline_key);
        if let Some(pipeline) = self.resources.graphics_pipelines.get(hash, &pipeline_key) {
            Ok(pipeline)
        } else {
            println!("Creating pipeline\n{:#?}", pipeline_key);
            let resource = crate::pipeline_description::create_graphics_pipeline(
                &self.device_context.device(),
                &pipeline_asset.fixed_function_state,
                pipeline_create_data.pipeline_layout.get_raw(),
                renderpass.get_raw(),
                &pipeline_create_data.shader_module_metas,
                &pipeline_create_data.shader_module_vk_objs,
                swapchain_surface_info
            )?;
            println!("Created pipeline {:?}", resource);

            let pipeline = self.resources.graphics_pipelines.insert(hash, &pipeline_key, resource);
            Ok(pipeline)
        }
    }

    fn load_graphics_pipeline(
        &mut self,
        pipeline_asset: &PipelineAsset,
    ) -> VkResult<LoadedGraphicsPipeline> {
        //TODO: Hashing the asset comes with the downside that if shader assets are different load
        // handles but the same values, we don't deduplicate them.
        let pipeline_create_data = PipelineCreateData::new(self, pipeline_asset)?;
        let swapchain_surface_infos = self.swapchain_surfaces.unique_swapchain_infos().clone();
        let mut render_passes = Vec::with_capacity(self.swapchain_surfaces.unique_swapchain_infos().len());
        for swapchain_surface_info in &swapchain_surface_infos {
            let render_pass = self.get_or_create_renderpass(&pipeline_asset.renderpass, swapchain_surface_info)?;
            render_passes.push(render_pass);
        }

        //
        // Render passes and pipelines
        //
        let mut pipelines = Vec::with_capacity(self.swapchain_surfaces.unique_swapchain_infos().len());

        for (swapchain_surface_info, renderpass) in swapchain_surface_infos.iter().zip(&render_passes) {
            let pipeline = self.get_or_create_pipeline(
                &pipeline_create_data,
                pipeline_asset,
                swapchain_surface_info,
                renderpass.clone()
            )?;
            pipelines.push(pipeline);
        }

        Ok(LoadedGraphicsPipeline {
            shader_modules: pipeline_create_data.shader_module_arcs,
            descriptor_set_layouts: pipeline_create_data.descriptor_set_layout_arcs,
            pipeline_layout: pipeline_create_data.pipeline_layout,
            render_passes,
            pipelines,
            pipeline_asset: pipeline_asset.clone()
        })
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        unsafe {
            println!("Cleaning up resource manager");
            self.dump_stats();

            // Wipe out any loaded assets. This will potentially drop ref counts on resources
            self.loaded_assets.destroy();

            // Now drop all resources with a zero ref count and warn for any resources that remain
            self.resources.destroy(self.device_context.device());

            println!("Dropping resource manager");
            self.dump_stats();
        }
    }
}

// We have to create pipelines when pipeline assets load and when swapchains are added/removed.
// Gathering all the info to hash and create a pipeline is a bit involved so we share the code
// here
struct PipelineCreateData {
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_load_handles: Vec<LoadHandle>,
    shader_module_arcs: Vec<ResourceArc<vk::ShaderModule>>,
    shader_module_vk_objs: Vec<vk::ShaderModule>,

    descriptor_set_layout_arcs: Vec<ResourceArc<vk::DescriptorSetLayout>>,

    pipeline_layout: ResourceArc<vk::PipelineLayout>,
}

impl PipelineCreateData {
    pub fn new(
        resource_manager: &mut ResourceManager,
        pipeline_asset: &PipelineAsset,
    ) -> VkResult<Self> {
        //
        // Shader module metadata (required to create the pipeline key)
        //
        let mut shader_module_metas = Vec::with_capacity(pipeline_asset.pipeline_shader_stages.len());
        let mut shader_module_load_handles = Vec::with_capacity(pipeline_asset.pipeline_shader_stages.len());
        for stage in &pipeline_asset.pipeline_shader_stages {
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
        let mut shader_module_arcs = Vec::with_capacity(pipeline_asset.pipeline_shader_stages.len());
        let mut shader_module_vk_objs = Vec::with_capacity(pipeline_asset.pipeline_shader_stages.len());
        for stage in &pipeline_asset.pipeline_shader_stages {
            let shader_module = resource_manager.loaded_assets.shader_modules.get_latest(stage.shader_module.load_handle()).unwrap();
            shader_module_arcs.push(shader_module.shader_module.clone());
            shader_module_vk_objs.push(shader_module.shader_module.get_raw());
        }

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layout_arcs = Vec::with_capacity(pipeline_asset.pipeline_layout.descriptor_set_layouts.len());
        for descriptor_set_layout_def in &pipeline_asset.pipeline_layout.descriptor_set_layouts {
            let descriptor_set_layout = resource_manager.get_or_create_descriptor_set_layout(descriptor_set_layout_def)?;
            descriptor_set_layout_arcs.push(descriptor_set_layout);
        }

        //
        // Pipeline layout
        //
        let pipeline_layout = resource_manager.get_or_create_pipeline_layout(&pipeline_asset.pipeline_layout)?;

        Ok(PipelineCreateData {
            shader_module_metas,
            shader_module_load_handles,
            shader_module_arcs,
            shader_module_vk_objs,
            descriptor_set_layout_arcs,
            pipeline_layout
        })
    }
}






