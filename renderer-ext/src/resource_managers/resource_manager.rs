use std::sync::Arc;
use std::sync::Weak;
use crossbeam_channel::{Sender, Receiver};
use atelier_assets::loader::LoadHandle;
use fnv::FnvHashMap;
use ash::{vk, Device};
use crate::pipeline_description as dsc;
use renderer_shell_vulkan::{VkResource, VkResourceDropSink, VkDeviceContext, VkPoolAllocator, VkDescriptorPoolAllocator, VkImage, VkImageRaw};
use std::hash::Hash;
use std::marker::PhantomData;
use atelier_assets::loader::handle::AssetHandle;

//TODO: Add the concept of vertex streams?


// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct ResourceHash(u64);

impl ResourceHash {
    pub fn from_key<KeyT: Hash>(key: &KeyT) -> ResourceHash {
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
    ResourceT: Copy
{
    resource: ResourceT,
    resource_hash: ResourceHash,
}

impl<ResourceT> std::fmt::Debug for ResourceWithHash<ResourceT>
    where
        ResourceT: std::fmt::Debug + Copy
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceWithHash")
            .field("resource", &self.resource)
            .field("resource_hash", &self.resource_hash)
            .finish()
    }
}

struct ResourceArcInner<ResourceT>
where
    ResourceT: Copy
{
    resource: ResourceWithHash<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
}

impl<ResourceT> Drop for ResourceArcInner<ResourceT>
where
    ResourceT: Copy
{
    fn drop(&mut self) {
        self.drop_tx.send(self.resource.clone());
    }
}

impl<ResourceT> std::fmt::Debug for ResourceArcInner<ResourceT>
    where
        ResourceT: std::fmt::Debug + Copy
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceArcInner")
            .field("resource", &self.resource)
            .finish()
    }
}

#[derive(Clone)]
pub struct WeakResourceArc<ResourceT>
where
    ResourceT: Copy
{
    inner: Weak<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> WeakResourceArc<ResourceT>
where
    ResourceT: Copy
{
    pub fn upgrade(&self) -> Option<ResourceArc<ResourceT>> {
        if let Some(upgrade) = self.inner.upgrade() {
            Some(ResourceArc { inner: upgrade })
        } else {
            None
        }
    }
}

impl<ResourceT> std::fmt::Debug for WeakResourceArc<ResourceT>
    where
        ResourceT: std::fmt::Debug + Copy
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeakResourceArc")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone)]
pub struct ResourceArc<ResourceT>
where
    ResourceT: Copy
{
    inner: Arc<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> ResourceArc<ResourceT>
where
    ResourceT: Copy
{
    fn new(
        resource: ResourceT,
        resource_hash: ResourceHash,
        drop_tx: Sender<ResourceWithHash<ResourceT>>,
    ) -> Self {
        ResourceArc {
            inner: Arc::new(ResourceArcInner {
                resource: ResourceWithHash {
                    resource,
                    resource_hash,
                },
                drop_tx,
            }),
        }
    }

    pub fn get_raw(&self) -> ResourceT {
        self.inner.resource.borrow().resource
    }

    pub fn downgrade(&self) -> WeakResourceArc<ResourceT> {
        let inner = Arc::downgrade(&self.inner);
        WeakResourceArc { inner }
    }
}

impl<ResourceT> std::fmt::Debug for ResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Copy
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceArc")
            .field("inner", &self.inner)
            .finish()
    }
}

//
// A lookup of resources. They reference count using Arcs internally and send a signal when they
// drop. This allows the resources to be collected and disposed of
//
struct ResourceLookup<KeyT, ResourceT>
where
    KeyT: Eq + Hash + Clone,
    ResourceT: VkResource + Copy,
{
    resources: FnvHashMap<ResourceHash, WeakResourceArc<ResourceT>>,
    drop_sink: VkResourceDropSink<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
    drop_rx: Receiver<ResourceWithHash<ResourceT>>,
    phantom_data: PhantomData<KeyT>,
    #[cfg(debug_assertions)]
    keys: FnvHashMap<ResourceHash, KeyT>,
}

impl<KeyT, ResourceT> ResourceLookup<KeyT, ResourceT>
where
    KeyT: Eq + Hash + Clone,
    ResourceT: VkResource + Copy + std::fmt::Debug,
{
    fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        ResourceLookup {
            resources: Default::default(),
            drop_sink: VkResourceDropSink::new(max_frames_in_flight),
            drop_tx,
            drop_rx,
            phantom_data: Default::default(),
            #[cfg(debug_assertions)]
            keys: Default::default(),
        }
    }

    fn get(
        &self,
        hash: ResourceHash,
        key: &KeyT,
    ) -> Option<ResourceArc<ResourceT>> {
        if let Some(resource) = self.resources.get(&hash) {
            let upgrade = resource.upgrade();

            #[cfg(debug_assertions)]
            if upgrade.is_some() {
                debug_assert!(self.keys.get(&hash).unwrap() == key);
            }

            upgrade
        } else {
            None
        }
    }

    fn insert(
        &mut self,
        hash: ResourceHash,
        key: &KeyT,
        resource: ResourceT,
    ) -> ResourceArc<ResourceT> {
        // Process any pending drops. If we don't do this, it's possible that the pending drop could
        // wipe out the state we're about to set
        self.handle_dropped_resources();

        println!(
            "insert resource {} {:?}",
            core::any::type_name::<ResourceT>(),
            resource
        );

        let arc = ResourceArc::new(resource, hash, self.drop_tx.clone());
        let downgraded = arc.downgrade();
        let old = self.resources.insert(hash, downgraded);
        assert!(old.is_none());

        #[cfg(debug_assertions)]
        {
            self.keys.insert(hash, key.clone());
            assert!(old.is_none());
        }

        arc
    }

    fn handle_dropped_resources(&mut self) {
        for dropped in self.drop_rx.try_iter() {
            println!(
                "dropping {} {:?}",
                core::any::type_name::<ResourceT>(),
                dropped.resource
            );
            self.drop_sink.retire(dropped.resource);
            self.resources.remove(&dropped.resource_hash);

            #[cfg(debug_assertions)]
            {
                self.keys.remove(&dropped.resource_hash);
            }
        }
    }

    fn len(&self) -> usize {
        self.resources.len()
    }

    fn on_frame_complete(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        self.handle_dropped_resources();
        self.drop_sink.on_frame_complete(device_context);
    }

    fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) {
        self.handle_dropped_resources();

        if self.resources.len() > 0 {
            log::warn!(
                "{} resource count {} > 0, resources will leak",
                core::any::type_name::<ResourceT>(),
                self.resources.len()
            );
        }

        self.drop_sink.destroy(device_context);
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
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GraphicsPipelineKey {
    pipeline_layout: dsc::PipelineLayout,
    renderpass: dsc::RenderPass,
    fixed_function_state: dsc::FixedFunctionState,
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_load_handles: Vec<LoadHandle>,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ImageKey {
    load_handle: LoadHandle,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ImageViewKey {
    image_load_handle: LoadHandle,
    image_view_meta: dsc::ImageViewMeta
}

#[derive(Debug)]
struct ResourceMetrics {
    shader_module_count: usize,
    descriptor_set_layout_count: usize,
    pipeline_layout_count: usize,
    renderpass_count: usize,
    pipeline_count: usize,
    image_count: usize,
    image_view_count: usize,
    sampler_count: usize,
}

//
// Handles raw lookup and destruction of GPU resources. Everything is reference counted. No safety
// is provided for dependencies/order of destruction. The general expectation is that anything
// dropped can safely be destroyed after a few frames have passed (based on max number of frames
// that can be submitted to the GPU)
//
struct ResourceLookupSet {
    pub device_context: VkDeviceContext,
    pub shader_modules: ResourceLookup<dsc::ShaderModule, vk::ShaderModule>,
    pub descriptor_set_layouts: ResourceLookup<dsc::DescriptorSetLayout, vk::DescriptorSetLayout>,
    pub pipeline_layouts: ResourceLookup<dsc::PipelineLayout, vk::PipelineLayout>,
    pub render_passes: ResourceLookup<RenderPassKey, vk::RenderPass>,
    pub graphics_pipelines: ResourceLookup<GraphicsPipelineKey, vk::Pipeline>,
    pub images: ResourceLookup<ImageKey, VkImageRaw>,
    pub image_views: ResourceLookup<ImageViewKey, vk::ImageView>,
    pub samplers: ResourceLookup<dsc::Sampler, vk::Sampler>,
}

impl ResourceLookupSet {
    pub fn new(device_context: &VkDeviceContext, max_frames_in_flight: u32) -> Self {
        ResourceLookupSet {
            device_context: device_context.clone(),
            shader_modules: ResourceLookup::new(max_frames_in_flight),
            descriptor_set_layouts: ResourceLookup::new(max_frames_in_flight),
            pipeline_layouts: ResourceLookup::new(max_frames_in_flight),
            render_passes: ResourceLookup::new(max_frames_in_flight),
            graphics_pipelines: ResourceLookup::new(max_frames_in_flight),
            images: ResourceLookup::new(max_frames_in_flight),
            image_views: ResourceLookup::new(max_frames_in_flight),
            samplers: ResourceLookup::new(max_frames_in_flight),
        }
    }

    fn on_frame_complete(
        &mut self,
    ) {
        self.shader_modules.on_frame_complete(&self.device_context);
        self.descriptor_set_layouts.on_frame_complete(&self.device_context);
        self.pipeline_layouts.on_frame_complete(&self.device_context);
        self.render_passes.on_frame_complete(&self.device_context);
        self.graphics_pipelines.on_frame_complete(&self.device_context);
        self.images.on_frame_complete(&self.device_context);
        self.image_views.on_frame_complete(&self.device_context);
        self.samplers.on_frame_complete(&self.device_context);
    }

    fn destroy(
        &mut self,
    ) {
        self.shader_modules.destroy(&self.device_context);
        self.descriptor_set_layouts.destroy(&self.device_context);
        self.pipeline_layouts.destroy(&self.device_context);
        self.render_passes.destroy(&self.device_context);
        self.graphics_pipelines.destroy(&self.device_context);
        self.images.destroy(&self.device_context);
        self.image_views.destroy(&self.device_context);
        self.samplers.destroy(&self.device_context);
    }

    fn metrics(
        &self
    ) -> ResourceMetrics {
        ResourceMetrics {
            shader_module_count: self.shader_modules.len(),
            descriptor_set_layout_count: self.descriptor_set_layouts.len(),
            pipeline_layout_count: self.pipeline_layouts.len(),
            renderpass_count: self.render_passes.len(),
            pipeline_count: self.graphics_pipelines.len(),
            image_count: self.images.len(),
            image_view_count: self.image_views.len(),
            sampler_count: self.samplers.len(),
        }
    }

    fn get_or_create_shader_module(
        &mut self,
        shader_module: &dsc::ShaderModule,
    ) -> VkResult<ResourceArc<vk::ShaderModule>> {
        let hash = ResourceHash::from_key(shader_module);
        if let Some(shader_module) = self.shader_modules.get(hash, shader_module) {
            Ok(shader_module)
        } else {
            println!(
                "Creating shader module\n[bytes: {}]",
                shader_module.code.len()
            );
            let resource = crate::pipeline_description::create_shader_module(
                self.device_context.device(),
                shader_module,
            )?;
            println!("Created shader module {:?}", resource);
            let shader_module = self
                .shader_modules
                .insert(hash, shader_module, resource);
            Ok(shader_module)
        }
    }

    fn get_or_create_descriptor_set_layout(
        &mut self,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<ResourceArc<vk::DescriptorSetLayout>> {
        let hash = ResourceHash::from_key(descriptor_set_layout);
        if let Some(descriptor_set_layout) = self
            .descriptor_set_layouts
            .get(hash, descriptor_set_layout)
        {
            Ok(descriptor_set_layout)
        } else {
            println!(
                "Creating descriptor set layout\n{:#?}",
                descriptor_set_layout
            );
            let resource = crate::pipeline_description::create_descriptor_set_layout(
                self.device_context.device(),
                descriptor_set_layout,
            )?;
            println!("Created descriptor set layout {:?}", resource);
            let descriptor_set_layout =
                self.descriptor_set_layouts
                    .insert(hash, descriptor_set_layout, resource);
            Ok(descriptor_set_layout)
        }
    }

    fn get_or_create_pipeline_layout(
        &mut self,
        pipeline_layout_def: &dsc::PipelineLayout,
    ) -> VkResult<ResourceArc<vk::PipelineLayout>> {
        let hash = ResourceHash::from_key(pipeline_layout_def);
        if let Some(pipeline_layout) = self
            .pipeline_layouts
            .get(hash, pipeline_layout_def)
        {
            Ok(pipeline_layout)
        } else {
            // Keep both the arcs and build an array of vk object pointers
            let mut descriptor_set_layout_arcs =
                Vec::with_capacity(pipeline_layout_def.descriptor_set_layouts.len());
            let mut descriptor_set_layouts =
                Vec::with_capacity(pipeline_layout_def.descriptor_set_layouts.len());

            for descriptor_set_layout_def in &pipeline_layout_def.descriptor_set_layouts {
                let loaded_descriptor_set_layout =
                    self.get_or_create_descriptor_set_layout(descriptor_set_layout_def)?;
                descriptor_set_layout_arcs.push(loaded_descriptor_set_layout.clone());
                descriptor_set_layouts.push(loaded_descriptor_set_layout.get_raw());
            }

            println!("Creating pipeline layout\n{:#?}", pipeline_layout_def);
            let resource = crate::pipeline_description::create_pipeline_layout(
                self.device_context.device(),
                pipeline_layout_def,
                &descriptor_set_layouts,
            )?;
            println!("Created pipeline layout {:?}", resource);
            let pipeline_layout =
                self.pipeline_layouts
                    .insert(hash, pipeline_layout_def, resource);

            Ok(pipeline_layout)
        }
    }

    fn get_or_create_renderpass(
        &mut self,
        renderpass: &dsc::RenderPass,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> VkResult<ResourceArc<vk::RenderPass>> {
        let renderpass_key = RenderPassKey {
            dsc: renderpass.clone(),
            swapchain_surface_info: swapchain_surface_info.clone(),
        };

        let hash = ResourceHash::from_key(&renderpass_key);
        if let Some(renderpass) = self.render_passes.get(hash, &renderpass_key) {
            Ok(renderpass)
        } else {
            let resource = crate::pipeline_description::create_renderpass(
                self.device_context.device(),
                renderpass,
                &swapchain_surface_info,
            )?;

            let renderpass = self
                .render_passes
                .insert(hash, &renderpass_key, resource);
            Ok(renderpass)
        }
    }

    fn get_or_create_graphics_pipeline(
        &mut self,
        pipeline_create_data: &PipelineCreateData,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> VkResult<(ResourceArc<vk::RenderPass>, ResourceArc<vk::Pipeline>)> {
        let pipeline_key = GraphicsPipelineKey {
            shader_module_load_handles: pipeline_create_data.shader_module_load_handles.clone(),
            shader_module_metas: pipeline_create_data.shader_module_metas.clone(),
            pipeline_layout: pipeline_create_data.pipeline_layout_def.clone(),
            fixed_function_state: pipeline_create_data.fixed_function_state.clone(),
            renderpass: pipeline_create_data.renderpass.clone(),
            swapchain_surface_info: swapchain_surface_info.clone(),
        };

        let renderpass = self.get_or_create_renderpass(&pipeline_create_data.renderpass, swapchain_surface_info)?;

        let hash = ResourceHash::from_key(&pipeline_key);
        if let Some(pipeline) = self.graphics_pipelines.get(hash, &pipeline_key) {
            Ok((renderpass, pipeline))
        } else {

            println!("Creating pipeline\n{:#?}", pipeline_key);
            let resource = crate::pipeline_description::create_graphics_pipeline(
                &self.device_context.device(),
                &pipeline_create_data.fixed_function_state,
                pipeline_create_data.pipeline_layout.get_raw(),
                renderpass.get_raw(),
                &pipeline_create_data.shader_module_metas,
                &pipeline_create_data.shader_module_vk_objs,
                swapchain_surface_info,
            )?;
            println!("Created pipeline {:?}", resource);

            let pipeline = self
                .graphics_pipelines
                .insert(hash, &pipeline_key, resource);
            Ok((renderpass, pipeline))
        }
    }

    fn insert_image(&mut self, load_handle: LoadHandle, image: ManuallyDrop<VkImage>) -> ResourceArc<VkImageRaw> {
        let image_key = ImageKey {
            load_handle
        };

        let hash = ResourceHash::from_key(&image_key);
        let raw_image = ManuallyDrop::into_inner(image).take_raw().unwrap();
        self.images.insert(hash, &image_key, raw_image)
    }

    fn get_or_create_image_view(
        &mut self,
        image_load_handle: LoadHandle,
        image_view_meta: &dsc::ImageViewMeta,
    ) -> VkResult<ResourceArc<vk::ImageView>> {
        let image_view_key = ImageViewKey {
            image_load_handle,
            image_view_meta: image_view_meta.clone(),
        };

        let hash = ResourceHash::from_key(&image_view_key);
        if let Some(image_view) = self.image_views.get(hash, &image_view_key) {
            Ok(image_view)
        } else {

            let image_key = ImageKey {
                load_handle: image_load_handle
            };
            let image_load_handle_hash = ResourceHash::from_key(&image_load_handle);
            let image = self.images.get(image_load_handle_hash, &image_key).unwrap();

            println!("Creating image view\n{:#?}", image_view_key);
            let resource = crate::pipeline_description::create_image_view(
                &self.device_context.device(),
                image.get_raw().image,
                image_view_meta,
            )?;
            println!("Created image view\n{:#?}", resource);

            let image_view = self
                .image_views
                .insert(hash, &image_view_key, resource);
            Ok(image_view)
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
    shader_module: ResourceArc<vk::ShaderModule>,
}

// The actual GPU resources are held in Material because the pipeline does not specify everything
// needed to create the pipeline
struct LoadedGraphicsPipeline2 {
    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pipeline_asset: PipelineAsset2,
}

struct SlotLocation {
    pub layout_index: u32,
    pub binding_index: u32,
}

struct LoadedMaterialPass {
    shader_modules: Vec<ResourceArc<vk::ShaderModule>>,
    descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>,
    pipeline_layout: ResourceArc<vk::PipelineLayout>,

    // Potentially one of these per swapchain surface
    render_passes: Vec<ResourceArc<vk::RenderPass>>,
    pipelines: Vec<ResourceArc<vk::Pipeline>>,

    // We need to keep a copy of the asset so that we can recreate the pipeline for new swapchains
    pipeline_create_data: PipelineCreateData,

    //descriptor_set_factory: DescriptorSetFactory,
    shader_interface: MaterialPassShaderInterface,

    //TODO: Use hash instead of string. Probably want to have a "hashed string" type that keeps the
    // string around only in debug mode. Maybe this could be generalized to a HashOfThing<T>.
    pass_slot_name_lookup: FnvHashMap<String, Vec<SlotLocation>>,
}

struct LoadedMaterial {
    passes: Vec<LoadedMaterialPass>,

}

struct LoadedMaterialInstance {
    material_descriptor_sets: Vec<Vec<DescriptorSetArc>>
}

struct LoadedImage {
    //image_load_handle: LoadHandle,
    //image_view_meta: dsc::ImageViewMeta,
    image: ResourceArc<VkImageRaw>,
    image_view: ResourceArc<vk::ImageView>,

    // One per swapchain
    //image_views: Vec<ResourceArc<vk::ImageView>>

}

//
// Represents a single asset which may simultaneously have committed and uncommitted loaded state
//
struct LoadedAssetState<LoadedAssetT> {
    committed: Option<LoadedAssetT>,
    uncommitted: Option<LoadedAssetT>,
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
    //TODO: Slab these for faster lookup?
    loaded_assets: FnvHashMap<LoadHandle, LoadedAssetState<LoadedAssetT>>,
}

impl<LoadedAssetT> AssetLookup<LoadedAssetT> {
    fn set_uncommitted(
        &mut self,
        load_handle: LoadHandle,
        loaded_asset: LoadedAssetT,
    ) {
        self.loaded_assets
            .entry(load_handle)
            .or_default()
            .uncommitted = Some(loaded_asset);
    }

    fn commit(
        &mut self,
        load_handle: LoadHandle,
    ) {
        let state = self.loaded_assets.get_mut(&load_handle).unwrap();
        state.committed = state.uncommitted.take();
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
    ) {
        let old = self.loaded_assets.remove(&load_handle);
        assert!(old.is_some());
    }

    fn get_latest(
        &self,
        load_handle: LoadHandle,
    ) -> Option<&LoadedAssetT> {
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

    fn get_committed(
        &self,
        load_handle: LoadHandle,
    ) -> Option<&LoadedAssetT> {
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
        self.loaded_assets.clear();
    }
}

impl<LoadedAssetT> Default for AssetLookup<LoadedAssetT> {
    fn default() -> Self {
        AssetLookup {
            loaded_assets: Default::default(),
        }
    }
}

//
// Lookups by asset for loaded asset state
//
#[derive(Default)]
struct LoadedAssetLookupSet {
    pub shader_modules: AssetLookup<LoadedShaderModule>,
    pub graphics_pipelines2: AssetLookup<LoadedGraphicsPipeline2>,
    pub materials: AssetLookup<LoadedMaterial>,
    pub material_instances: AssetLookup<LoadedMaterialInstance>,
    pub images: AssetLookup<LoadedImage>
}

impl LoadedAssetLookupSet {
    pub fn destroy(&mut self) {
        self.shader_modules.destroy();
        self.graphics_pipelines2.destroy();
        self.materials.destroy();
        self.material_instances.destroy();
        self.images.destroy();
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
    phantom_data: PhantomData<T>,
}

pub struct FreeRequest<T> {
    load_handle: LoadHandle,
    phantom_data: PhantomData<T>,
}

pub struct LoadQueuesTx<T> {
    load_request_tx: Sender<LoadRequest<T>>,
    commit_request_tx: Sender<CommitRequest<T>>,
    free_request_tx: Sender<FreeRequest<T>>,
}

impl<T> Clone for LoadQueuesTx<T> {
    fn clone(&self) -> Self {
        LoadQueuesTx {
            load_request_tx: self.load_request_tx.clone(),
            commit_request_tx: self.commit_request_tx.clone(),
            free_request_tx: self.free_request_tx.clone(),
        }
    }
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

impl<T> LoadQueues<T>
{
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

impl<T> LoadQueues<T>
    where T:  TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone
{
    pub fn create_load_handler(&self) -> GenericLoadHandler<T> {
        GenericLoadHandler {
            load_queues: self.tx.clone()
        }
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
            free_request_tx,
        };

        let rx = LoadQueuesRx {
            load_request_rx,
            commit_request_rx,
            free_request_rx,
        };

        LoadQueues { tx, rx }
    }
}

//
// A generic load handler that allows routing load/commit/free events
//
use crate::asset_storage::ResourceHandle;
use crate::asset_storage::ResourceLoadHandler;
use type_uuid::TypeUuid;
use crate::pipeline::pipeline::{MaterialAsset2, PipelineAsset2, MaterialPass, MaterialInstanceAsset2, DescriptorSetLayoutBindingWithSlotName, MaterialPassShaderInterface};
use crate::pipeline::shader::ShaderAsset;
use ash::prelude::VkResult;
use crate::pipeline_description::{SwapchainSurfaceInfo, DescriptorType};
use std::borrow::Borrow;
use atelier_assets::loader::handle::Handle;
use crate::asset_resource::AssetResource;
use ash::version::DeviceV1_0;
use std::collections::VecDeque;
use itertools::all;
use renderer_base::slab::{RawSlab, RawSlabKey};
use std::cmp::max;
use crate::pipeline::image::ImageAsset;
use crate::upload::{UploadQueue, PendingImageUpload, ImageUploadOpResult, BufferUploadOpResult, UploadOp};
use crate::image_utils::DecodedTexture;
use image::load;
use std::mem::ManuallyDrop;
use serde::export::Formatter;
use std::num::Wrapping;

pub struct GenericLoadHandler<AssetT>
where
    AssetT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
{
    load_queues: LoadQueuesTx<AssetT>,
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
        load_op: AssetLoadOp,
    ) {
        println!("ResourceLoadHandler update_asset {} {:?}", core::any::type_name::<AssetT>(), load_handle);
        let request = LoadRequest {
            load_handle,
            load_op,
            asset: asset.clone(),
        };

        self.load_queues.load_request_tx.send(request);
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        resource_handle: ResourceHandle<AssetT>,
        version: u32,
        asset: &AssetT,
    ) {
        println!("ResourceLoadHandler commit_asset_version {} {:?}", core::any::type_name::<AssetT>(), load_handle);
        let request = CommitRequest {
            load_handle,
            phantom_data: Default::default(),
        };

        self.load_queues.commit_request_tx.send(request);
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        resource_handle: ResourceHandle<AssetT>,
        version: u32,
    ) {
        println!("ResourceLoadHandler free {} {:?}", core::any::type_name::<AssetT>(), load_handle);
        let request = FreeRequest {
            load_handle,
            phantom_data: Default::default(),
        };

        self.load_queues.free_request_tx.send(request);
    }
}

#[derive(Default)]
struct LoadQueueSet {
    shader_modules: LoadQueues<ShaderAsset>,
    graphics_pipelines2: LoadQueues<PipelineAsset2>,
    materials: LoadQueues<MaterialAsset2>,
    material_instances: LoadQueues<MaterialInstanceAsset2>,
    images: LoadQueues<ImageAsset>
}

pub struct ActiveSwapchainSurfaceInfoState {
    ref_count: u32,
    index: usize,
}

#[derive(Default)]
pub struct ActiveSwapchainSurfaceInfoSet {
    ref_counts: FnvHashMap<dsc::SwapchainSurfaceInfo, ActiveSwapchainSurfaceInfoState>,

    //TODO: Could make this a slab which would persist indexes across frames
    unique_swapchain_infos: Vec<dsc::SwapchainSurfaceInfo>,
}

impl ActiveSwapchainSurfaceInfoSet {
    fn add(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> bool {
        match self.ref_counts.get_mut(swapchain_surface_info) {
            Some(state) => {
                state.ref_count += 1;
                false
            }
            None => {
                &self.ref_counts.insert(
                    swapchain_surface_info.clone(),
                    ActiveSwapchainSurfaceInfoState {
                        ref_count: 1,
                        index: self.unique_swapchain_infos.len(),
                    },
                );

                self.unique_swapchain_infos
                    .push(swapchain_surface_info.clone());
                true
            }
        }
    }

    fn remove(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> Option<usize> {
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
            }
            // If it doesn't exist, then a remove call was made before a matching add call
            None => unreachable!(),
        }
    }

    pub fn unique_swapchain_infos(&self) -> &Vec<dsc::SwapchainSurfaceInfo> {
        &self.unique_swapchain_infos
    }
}














//TODO:
// - MaterialInstancePool, MaterialInstancePoolChunk
// - How to index into these? Need to defrag them?
// - Batch the updates
// - Can't modify a descriptor set that's in flight, may need to copy-on-write?
// Process:
// - (DONE) Load a material instance asset, which pushes create/update/free through the load queues
// - (DONE) Drain create events from the load queues and create a LoadedMaterial in the loaded asset manager
// - (DONE) The loaded material will need a ResourceArc to represent the actual instance
// - (DONE) The ResourceArc is created by calling a function on the MaterialInstancePool passing the
//   material instance data. This may include data fetched from the resource lookup
// - (DONE) The payload in the ResourceArc should contain the index
// - (DONE) The index is generated by the MaterialInstancePool and promised to be unique among all existing
//   ResourceArcs in the same pool
// - Ideally we want to batch updating the descriptor sets but this means the vk::DescriptorSet isn't
//   ready to be used until after the updates occur
// - May need to flush the updates immediately after loading material instances - which ensures any
//   loading asset that depends on a material
// - When the ResourceArc is dropped (maybe because the asset unloads) a message is pushed into
//   the material instance pool destroy queue
// - Fetch the LoadedMaterial
// - Update/Destroy queues for material instances
// - Create a loaded material instance with a ResourceArc<vk::DescriptorSet>



/*
enum MaterialSlotValues {
    //Image(ResourceArc<vk::Image>),
    Scalar(f32)
}


struct MaterialInstanceUpdate {
    image_updates: Vec<MaterialInstanceUpdate>
}

struct MaterialInstanceRemove {
    image_updates: Vec<MaterialInstanceUpdate>
}



struct MaterialInstancePoolChunk {
    pool: vk::DescriptorPool,
    free_list: Vec<u8>,
    update_queue: VecDeque<MaterialInstanceUpdate>,
    free_queue: VecDeque<MaterialInstanceRemove>
}

impl MaterialInstancePoolChunk {
    fn new(
        pool: vk::DescriptorPool,
    ) -> Self {
        MaterialInstancePoolChunk {
            pool,
            free_list: Default::default(),
        }
    }

    fn update(
        &mut self,
        device: &ash::Device,
        allocator: &mut VkDescriptorPoolAllocator,
    ) -> VkResult<()> {
        if self.update_queue.is_empty() && self.free_queue.is_empty() {
            return Ok(());
        }

        let new_pool = allocator.allocate_pool(device)?;




        allocator.retire_pool(self.pool);
        self.pool = new_pool;
    }
}


struct MaterialInstancePool {
    device_context: VkDeviceContext,
    allocator: VkDescriptorPoolAllocator,
    descriptor_set_layout_def: dsc::DescriptorSetLayout,
    descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
}

impl MaterialInstancePool {
    const DESCRIPTOR_COUNT_PER_CHUNK : u32 = 64;

    fn new(
        device_context: &VkDeviceContext,
        max_in_flight_frames: u32,
        descriptor_set_layout_def: dsc::DescriptorSetLayout,
        descriptor_set_layout: ResourceArc<vk::DescriptorSetLayout>,
        count_per_pool: u32,
    ) -> Self {
        let allocator_descriptor_set_layout_def = descriptor_set_layout_def.clone();
        let allocator = VkDescriptorPoolAllocator::new(
            max_in_flight_frames,
            std::u32::MAX,
            move |device| {
                //let layout_def = dsc::DescriptorSetLayout::default();
                Self::create_descriptor_pool(device, &allocator_descriptor_set_layout_def)
            }
        );

        MaterialInstancePool {
            device_context: device_context.clone(),
            allocator,
            descriptor_set_layout_def,
            descriptor_set_layout,
        }
    }


    fn create_descriptor_pool(
        device: &ash::Device,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
    ) -> VkResult<vk::DescriptorPool> {
        let pool_sizes = [vk::DescriptorPoolSize::builder()
            .ty(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(Self::DESCRIPTOR_COUNT_PER_CHUNK)
            .build()];

        let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(Self::DESCRIPTOR_COUNT_PER_CHUNK);

        unsafe { device.create_descriptor_pool(&descriptor_pool_info, None) }
    }

    fn allocate() {

    }
}




*/





//
// These represent a write update that can be applied to a descriptor set in a pool
//
#[derive(Debug)]
struct DescriptorSetWriteImage {
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
struct DescriptorSetWriteBuffer {
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
struct DescriptorSetWrite {
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

struct DescriptorSetArc {
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



struct RegisteredDescriptorSetPoolManager {
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










struct UploadManager {
    upload_queue: UploadQueue,

    image_upload_result_tx: Sender<ImageUploadOpResult>,
    image_upload_result_rx: Receiver<ImageUploadOpResult>,

    buffer_upload_result_tx: Sender<BufferUploadOpResult>,
    buffer_upload_result_rx: Receiver<BufferUploadOpResult>,
}

impl UploadManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        let (image_upload_result_tx, image_upload_result_rx) = crossbeam_channel::unbounded();
        let (buffer_upload_result_tx, buffer_upload_result_rx) = crossbeam_channel::unbounded();

        UploadManager {
            upload_queue: UploadQueue::new(device_context),
            image_upload_result_rx,
            image_upload_result_tx,
            buffer_upload_result_rx,
            buffer_upload_result_tx
        }
    }

    pub fn update(&mut self) -> VkResult<()> {
        self.upload_queue.update()
    }

    pub fn upload_image(&self, request: LoadRequest<ImageAsset>) -> VkResult<()> {
        let decoded_texture = DecodedTexture {
            width: request.asset.width,
            height: request.asset.height,
            data: request.asset.data,
        };

        self.upload_queue.pending_image_tx().send(PendingImageUpload {
            load_op: request.load_op,
            upload_op: UploadOp::new(request.load_handle, self.image_upload_result_tx.clone()),
            texture: decoded_texture
        }).map_err(|err| {
            log::error!("Could not enqueue image upload");
            vk::Result::ERROR_UNKNOWN
        })
    }
}














pub struct PipelineInfo {
    pub descriptor_set_layouts: Vec<ResourceArc<vk::DescriptorSetLayout>>,
    pub pipeline_layout: ResourceArc<vk::PipelineLayout>,
    pub renderpass: ResourceArc<vk::RenderPass>,
    pub pipeline: ResourceArc<vk::Pipeline>,
}

pub struct CurrentFramePassInfo {
    pub descriptor_sets: Vec<vk::DescriptorSet>
}

pub struct ResourceManager {
    device_context: VkDeviceContext,

    resources: ResourceLookupSet,
    loaded_assets: LoadedAssetLookupSet,
    load_queues: LoadQueueSet,
    swapchain_surfaces: ActiveSwapchainSurfaceInfoSet,
    registered_descriptor_sets: RegisteredDescriptorSetPoolManager,
    upload_manager: UploadManager,
}

impl ResourceManager {
    pub fn new(device_context: &VkDeviceContext) -> Self {
        ResourceManager {
            device_context: device_context.clone(),
            resources: ResourceLookupSet::new(device_context,renderer_shell_vulkan::MAX_FRAMES_IN_FLIGHT as u32),
            loaded_assets: Default::default(),
            load_queues: Default::default(),
            swapchain_surfaces: Default::default(),
            registered_descriptor_sets: RegisteredDescriptorSetPoolManager::new(device_context),
            upload_manager: UploadManager::new(device_context),
        }
    }

    pub fn create_shader_load_handler(&self) -> GenericLoadHandler<ShaderAsset> {
        self.load_queues.shader_modules.create_load_handler()
    }

    pub fn create_pipeline2_load_handler(&self) -> GenericLoadHandler<PipelineAsset2> {
        self.load_queues.graphics_pipelines2.create_load_handler()
    }

    pub fn create_material_load_handler(&self) -> GenericLoadHandler<MaterialAsset2> {
        self.load_queues.materials.create_load_handler()
    }

    pub fn create_material_instance_load_handler(&self) -> GenericLoadHandler<MaterialInstanceAsset2> {
        self.load_queues.material_instances.create_load_handler()
    }

    pub fn create_image_load_handler(&self) -> GenericLoadHandler<ImageAsset> {
        self.load_queues.images.create_load_handler()
    }

    pub fn get_pipeline_info(
        &self,
        handle: &Handle<MaterialAsset2>,
        swapchain: &SwapchainSurfaceInfo,
        pass_index: usize,
    ) -> PipelineInfo {
        let resource = self
            .loaded_assets
            .materials
            .get_committed(handle.load_handle())
            .unwrap();
        let swapchain_index = self
            .swapchain_surfaces
            .ref_counts
            .get(swapchain)
            .unwrap()
            .index;

        PipelineInfo {
            descriptor_set_layouts: resource.passes[pass_index].descriptor_set_layouts.clone(),
            pipeline_layout: resource.passes[pass_index].pipeline_layout.clone(),
            renderpass: resource.passes[pass_index].render_passes[swapchain_index].clone(),
            pipeline: resource.passes[pass_index].pipelines[swapchain_index].clone(),
        }
    }

    pub fn get_current_frame_pass_info(
        &self,
        handle: &Handle<MaterialInstanceAsset2>,
        pass_index: usize,
    ) -> CurrentFramePassInfo {
        let resource = self
            .loaded_assets
            .material_instances
            .get_committed(handle.load_handle())
            .unwrap();
        // let swapchain_index = self
        //     .swapchain_surfaces
        //     .ref_counts
        //     .get(swapchain)
        //     .unwrap()
        //     .index;

        // Get the current pass
        // Get the descriptor sets within the pass (one per layout)
        // Map the DescriptorSetArc to a vk::DescriptorSet
        let descriptor_sets : Vec<_> = resource.material_descriptor_sets[pass_index].iter()
            .map(|x| self.registered_descriptor_sets.descriptor_set(x))
            .collect();

        CurrentFramePassInfo {
            descriptor_sets
        }


        // PipelineInfo {
        //     descriptor_set_layouts: resource.passes[pass_index].descriptor_set_layouts.clone(),
        //     pipeline_layout: resource.passes[pass_index].pipeline_layout.clone(),
        //     renderpass: resource.passes[pass_index].render_passes[swapchain_index].clone(),
        //     pipeline: resource.passes[pass_index].pipelines[swapchain_index].clone(),
        // }
    }

    pub fn add_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) -> VkResult<()> {
        log::info!("add_swapchain {:?}", swapchain_surface_info);
        // Add it
        if self.swapchain_surfaces.add(&swapchain_surface_info) {

            for (load_handle, loaded_asset) in
            &mut self.loaded_assets.materials.loaded_assets
            {
                if let Some(committed) = &mut loaded_asset.committed {
                    for pass in &mut committed.passes {
                        let (renderpass, pipeline) = self.resources.get_or_create_graphics_pipeline(
                            &pass.pipeline_create_data,
                            swapchain_surface_info,
                        )?;

                        pass.render_passes.push(renderpass);
                        pass.pipelines.push(pipeline);
                    }
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    for pass in &mut uncommitted.passes {
                        let (renderpass, pipeline) = self.resources.get_or_create_graphics_pipeline(
                            &pass.pipeline_create_data,
                            swapchain_surface_info,
                        )?;

                        pass.render_passes.push(renderpass);
                        pass.pipelines.push(pipeline);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn remove_swapchain(
        &mut self,
        swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    ) {
        log::info!("remove_swapchain {:?}", swapchain_surface_info);
        //TODO: Common case is to destroy and re-create the same swapchain surface info, so we can
        // delay destroying until we also get an additional add/remove. If the next add call is
        // the same, we can avoid the remove entirely
        if let Some(remove_index) = self.swapchain_surfaces.remove(swapchain_surface_info) {
            for (_, loaded_asset) in &mut self.loaded_assets.materials.loaded_assets {
                if let Some(committed) = &mut loaded_asset.committed {
                    for pass in &mut committed.passes {
                        pass.render_passes.swap_remove(remove_index);
                        pass.pipelines.swap_remove(remove_index);
                    }
                }

                if let Some(uncommitted) = &mut loaded_asset.uncommitted {
                    for pass in &mut uncommitted.passes {
                        pass.render_passes.swap_remove(remove_index);
                        pass.pipelines.swap_remove(remove_index);
                    }
                }
            }
        } else {
            log::error!(
                "Received a remove swapchain without a matching add\n{:#?}",
                swapchain_surface_info
            );
        }
    }

    pub fn update(&mut self) -> VkResult<()> {
        self.process_shader_load_requests();
        self.process_pipeline_load_requests();
        self.process_material_load_requests();
        self.process_material_instance_load_requests();
        self.process_image_load_requests();

        self.registered_descriptor_sets.update();
        self.upload_manager.update()?;

        //self.dump_stats();

        Ok(())
    }

    fn dump_stats(&self) {
        let resource_metrics = self.resources.metrics();

        #[derive(Debug)]
        struct LoadedAssetCounts {
            shader_module_count: usize,
            pipeline2_count: usize,
            material_count: usize,
            material_instance_count: usize,
            image_count: usize,
        }

        let loaded_asset_counts = LoadedAssetCounts {
            shader_module_count: self.loaded_assets.shader_modules.len(),
            pipeline2_count: self.loaded_assets.graphics_pipelines2.len(),
            material_count: self.loaded_assets.materials.len(),
            material_instance_count: self.loaded_assets.material_instances.len(),
            image_count: self.loaded_assets.images.len(),
        };

        #[derive(Debug)]
        struct MaterialInstancePoolStats {
            hash: ResourceHash,
            allocated_count: usize,
        }

        #[derive(Debug)]
        struct RegisteredDescriptorSetStats {
            pools: Vec<MaterialInstancePoolStats>
        }

        let mut registered_descriptor_sets_stats = Vec::with_capacity(self.registered_descriptor_sets.pools.len());
        for (hash, value) in &self.registered_descriptor_sets.pools {
            registered_descriptor_sets_stats.push(MaterialInstancePoolStats {
                hash: *hash,
                allocated_count: value.slab.allocated_count()
            });
        }

        let registered_descriptor_sets_stats = RegisteredDescriptorSetStats {
            pools: registered_descriptor_sets_stats
        };

        #[derive(Debug)]
        struct ResourceManagerMetrics {
            resource_metrics: ResourceMetrics,
            loaded_asset_counts: LoadedAssetCounts,
            registered_descriptor_sets_stats: RegisteredDescriptorSetStats
        }

        let metrics = ResourceManagerMetrics {
            resource_metrics,
            loaded_asset_counts,
            registered_descriptor_sets_stats
        };

        println!("Resource Manager Metrics:\n{:#?}", metrics);
    }

    fn process_shader_load_requests(&mut self) {
        for request in self.load_queues.shader_modules.take_load_requests() {
            println!("Create shader module {:?}", request.load_handle);
            let loaded_asset = self.load_shader_module(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.shader_modules);
        }

        Self::handle_commit_requests(&mut self.load_queues.shader_modules, &mut self.loaded_assets.shader_modules);
        Self::handle_free_requests(&mut self.load_queues.shader_modules, &mut self.loaded_assets.shader_modules);
    }

    fn process_pipeline_load_requests(&mut self) {
        for request in self.load_queues.graphics_pipelines2.take_load_requests() {
            println!("Create pipeline2 {:?}", request.load_handle);
            let loaded_asset = self.load_graphics_pipeline(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.graphics_pipelines2);
        }

        Self::handle_commit_requests(&mut self.load_queues.graphics_pipelines2, &mut self.loaded_assets.graphics_pipelines2);
        Self::handle_free_requests(&mut self.load_queues.graphics_pipelines2, &mut self.loaded_assets.graphics_pipelines2);
    }

    fn process_material_load_requests(&mut self) {
        for request in self.load_queues.materials.take_load_requests() {
            println!("Create material {:?}", request.load_handle);
            let loaded_asset = self.load_material(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.materials);
        }

        Self::handle_commit_requests(&mut self.load_queues.materials, &mut self.loaded_assets.materials);
        Self::handle_free_requests(&mut self.load_queues.materials, &mut self.loaded_assets.materials);
    }

    fn process_material_instance_load_requests(&mut self) {
        for request in self.load_queues.material_instances.take_load_requests() {
            println!("Create material instance {:?}", request.load_handle);
            let loaded_asset = self.load_material_instance(&request.asset);
            Self::handle_load_result(request.load_op, loaded_asset, &mut self.loaded_assets.material_instances);
        }

        Self::handle_commit_requests(&mut self.load_queues.material_instances, &mut self.loaded_assets.material_instances);
        Self::handle_free_requests(&mut self.load_queues.material_instances, &mut self.loaded_assets.material_instances);
    }

    fn process_image_load_requests(&mut self) {
        for request in self.load_queues.images.take_load_requests() {
            //TODO: Route the request directly to the upload queue
            println!("Uploading image {:?}", request.load_handle);
            self.upload_manager.upload_image(request);
        }

        let results : Vec<_> = self.upload_manager.image_upload_result_rx.try_iter().collect();
        for result in results {
            match result {
                ImageUploadOpResult::UploadComplete(load_op, image) => {
                    let loaded_asset = self.finish_load_image(load_op.load_handle(), image);
                    Self::handle_load_result(load_op, loaded_asset, &mut self.loaded_assets.images);
                },
                ImageUploadOpResult::UploadError(load_handle) => {
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                },
                ImageUploadOpResult::UploadDrop(load_handle) => {
                    // Don't need to do anything - the uploaded should have triggered an error on the load_op
                }
            }
        }

        Self::handle_commit_requests(&mut self.load_queues.images, &mut self.loaded_assets.images);
        Self::handle_free_requests(&mut self.load_queues.images, &mut self.loaded_assets.images);
    }

    fn handle_load_result<LoadedAssetT>(
        load_op: AssetLoadOp,
        loaded_asset: VkResult<LoadedAssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>
    ) {
        match loaded_asset {
            Ok(loaded_asset) => {
                asset_lookup.set_uncommitted(load_op.load_handle(), loaded_asset);
                load_op.complete()
            }
            Err(err) => {
                load_op.error(err);
            }
        }
    }

    fn handle_commit_requests<AssetT, LoadedAssetT>(
        load_queues: &mut LoadQueues<AssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>
    ) {
        for request in load_queues.take_commit_requests() {
            asset_lookup.commit(request.load_handle);
        }
    }

    fn handle_free_requests<AssetT, LoadedAssetT>(
        load_queues: &mut LoadQueues<AssetT>,
        asset_lookup: &mut AssetLookup<LoadedAssetT>
    ) {
        for request in load_queues.take_commit_requests() {
            asset_lookup.commit(request.load_handle);
        }
    }

    fn finish_load_image(
        &mut self,
        image_load_handle: LoadHandle,
        image: ManuallyDrop<VkImage>,
    ) -> VkResult<LoadedImage> {
        let image = self.resources.insert_image(image_load_handle, image);

        let image_view_meta = dsc::ImageViewMeta {
            view_type: dsc::ImageViewType::Type2D,
            format: dsc::Format::R8G8B8A8_UNORM,
            subresource_range: dsc::ImageSubresourceRange {
                aspect_mask: dsc::ImageAspectFlags::Color,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1
            },
            components: dsc::ComponentMapping {
                r: dsc::ComponentSwizzle::Identity,
                g: dsc::ComponentSwizzle::Identity,
                b: dsc::ComponentSwizzle::Identity,
                a: dsc::ComponentSwizzle::Identity,
            }
        };

        let image_view = self.resources.get_or_create_image_view(image_load_handle, &image_view_meta)?;

        Ok(LoadedImage {
            image,
            image_view
        })
    }

    fn load_shader_module(
        &mut self,
        shader_module: &ShaderAsset,
    ) -> VkResult<LoadedShaderModule> {
        let shader_module = self.resources.get_or_create_shader_module(&shader_module.shader)?;
        Ok(LoadedShaderModule {
            shader_module
        })
    }

    fn load_graphics_pipeline(
        &mut self,
        pipeline_asset: &PipelineAsset2,
    ) -> VkResult<LoadedGraphicsPipeline2> {
        Ok(LoadedGraphicsPipeline2 {
            pipeline_asset: pipeline_asset.clone()
        })
    }

    fn load_material(
        &mut self,
        material_asset: &MaterialAsset2,
    ) -> VkResult<LoadedMaterial> {
        let mut passes = Vec::with_capacity(material_asset.passes.len());
        for pass in &material_asset.passes {
            let loaded_pipeline_asset = self.loaded_assets.graphics_pipelines2.get_latest(pass.pipeline.load_handle()).unwrap();
            let pipeline_asset = loaded_pipeline_asset.pipeline_asset.clone();

            let swapchain_surface_infos = self.swapchain_surfaces.unique_swapchain_infos().clone();
            let pipeline_create_data = PipelineCreateData::new(self, pipeline_asset, pass)?;

            // Will contain the vulkan resources being created per swapchain
            let mut render_passes =
                Vec::with_capacity(swapchain_surface_infos.len());
            let mut pipelines =
                Vec::with_capacity(swapchain_surface_infos.len());

            // Create the pipeline objects
            for swapchain_surface_info in swapchain_surface_infos
            {
                let (renderpass, pipeline) = self.resources.get_or_create_graphics_pipeline(
                    &pipeline_create_data,
                    &swapchain_surface_info,
                )?;
                render_passes.push(renderpass);
                pipelines.push(pipeline);
            }

            // Create a lookup of the slot names
            let mut pass_slot_name_lookup : FnvHashMap<String, Vec<SlotLocation>> = Default::default();
            for (layout_index, layout) in pass.shader_interface.descriptor_set_layouts.iter().enumerate() {
                for (binding_index, binding) in layout.descriptor_set_layout_bindings.iter().enumerate() {
                    pass_slot_name_lookup.entry(binding.slot_name.clone())
                        .or_default()
                        .push(SlotLocation {
                            layout_index: layout_index as u32,
                            binding_index: binding_index as u32,
                        });
                }
            }

            passes.push(LoadedMaterialPass {
                descriptor_set_layouts: pipeline_create_data.descriptor_set_layout_arcs.clone(),
                pipeline_layout: pipeline_create_data.pipeline_layout.clone(),
                shader_modules: pipeline_create_data.shader_module_arcs.clone(),
                render_passes,
                pipelines,
                pipeline_create_data,
                shader_interface: pass.shader_interface.clone(),
                pass_slot_name_lookup
            })
        }

        Ok(LoadedMaterial {
            passes,
        })
    }

    // fn begin_load_image(
    //     &mut self,
    //     request: LoadRequest<ImageAsset>
    // ) -> VkResult<LoadedMaterialInstance> {
    //     let decoded_image = DecodedTexture {
    //         width: request.asset.width,
    //         height: request.asset.height,
    //         data: request.asset.data,
    //     };
    //
    //     self.upload_manager.upload_queue.pending_image_tx().send(PendingImageUpload {
    //         load_op: request.load_op,
    //         upload_op: UploadOp::new(request.load_handle, self.upload_manager.image_upload_result_tx.clone()),
    //         texture: decoded_image
    //     }).map_err(|err| {
    //         log::error!("Could not enqueue image upload");
    //         vk::Result::ERROR_UNKNOWN
    //     })
    // }

    fn load_material_instance(
        &mut self,
        material_instance_asset: &MaterialInstanceAsset2,
    ) -> VkResult<LoadedMaterialInstance> {
        // Find the material we will bind over, we need the metadata from it
        let material_asset = self.loaded_assets.materials.get_latest(material_instance_asset.material.load_handle()).unwrap();

        //TODO: Validate the material instance's slot names exist somewhere in the material

        // This will be references to descriptor sets. Indexed by pass, and then by set within the pass.
        let mut material_descriptor_sets = Vec::with_capacity(material_asset.passes.len());
        for pass in &material_asset.passes {
            // The metadata for the descriptor sets within this pass, one for each set within the pass
            let descriptor_set_layouts = &pass.shader_interface.descriptor_set_layouts;// &pass.pipeline_create_data.pipeline_layout_def.descriptor_set_layouts;

            // This will contain the descriptor sets created for this pass, one for each set within the pass
            let mut pass_descriptor_sets = Vec::with_capacity(descriptor_set_layouts.len());

            // This will contain the writes for the descriptor set. Their purpose is to store everything needed to create a vk::WriteDescriptorSet
            // struct. We will need to keep these around for a few frames.
            let mut pass_descriptor_set_writes = Vec::with_capacity(pass.shader_interface.descriptor_set_layouts.len());

            //
            // Build a "default" descriptor writer for every binding
            //
            for layout in &pass.shader_interface.descriptor_set_layouts {
                // This will contain the writes for this set
                let mut layout_descriptor_set_writes = Vec::with_capacity(layout.descriptor_set_layout_bindings.len());

                for (binding_index, binding) in layout.descriptor_set_layout_bindings.iter().enumerate() {
                    //TODO: Populate the writer for this binding
                    //TODO: Allocate a set from the pool
                    // //pub dst_pool_index: u32, // a slab key?
                    // //pub dst_set_index: u32,
                    // pub dst_binding: u32,
                    // pub dst_array_element: u32,
                    // pub descriptor_type: vk::DescriptorType,
                    // pub image_info: Option<Vec<DescriptorSetWriteImage>>,
                    // pub buffer_info: Option<Vec<DescriptorSetWriteBuffer>>,
                    layout_descriptor_set_writes.push(DescriptorSetWrite {
                        // pool index
                        // set index
                        dst_binding: binding_index as u32,
                        dst_array_element: 0,
                        descriptor_type: binding.descriptor_type.into(),
                        image_info: Default::default(),
                        buffer_info: Default::default(),
                    })
                }

                pass_descriptor_set_writes.push(layout_descriptor_set_writes);
            }

            //
            // Now modify the descriptor set writes to actually point at the things specified by the material
            //
            for slot in &material_instance_asset.slots {
                if let Some(slot_locations) = pass.pass_slot_name_lookup.get(&slot.slot_name) {
                    for location in slot_locations {
                        let mut writer = &mut pass_descriptor_set_writes[location.layout_index as usize][location.binding_index as usize];

                        let mut bind_samplers = false;
                        let mut bind_images = false;
                        match writer.descriptor_type {
                            DescriptorType::Sampler => {
                                bind_samplers = true;
                            },
                            DescriptorType::CombinedImageSampler => {
                                bind_samplers = true;
                                bind_images = true;
                            },
                            DescriptorType::SampledImage => {
                                bind_images = true;
                            },
                            _ => { unimplemented!() }
                        }

                        let mut write_image = DescriptorSetWriteImage {
                            image_view: None,
                            sampler: None
                        };

                        if bind_images {
                            if let Some(image) = &slot.image {
                                let loaded_image = self.loaded_assets.images.get_latest(image.load_handle()).unwrap();
                                write_image.image_view = Some(loaded_image.image_view.clone());
                            }
                        }

                        writer.image_info = vec![write_image];
                    }
                }
            }

            //
            // Register the writes into the correct descriptor set pools
            //
            //let layouts = pass.pipeline_create_data.pipeline_layout.iter().zip(&pass.pipeline_create_data.pipeline_layout_def);
            for (layout_index, layout_writes) in pass_descriptor_set_writes.into_iter().enumerate() {
                let descriptor_set = self.registered_descriptor_sets.insert(
                    &pass.pipeline_create_data.pipeline_layout_def.descriptor_set_layouts[layout_index],
                    pass.pipeline_create_data.descriptor_set_layout_arcs[layout_index].clone(),
                    layout_writes,
                )?;

                pass_descriptor_sets.push(descriptor_set);
            }

            material_descriptor_sets.push(pass_descriptor_sets);
        }

        println!("MATERIAL SET\n{:#?}", material_descriptor_sets);

        Ok(LoadedMaterialInstance {
            material_descriptor_sets
        })
    }
}









//
//
//

// struct PendingDescriptorSetWriteImage {
//
//     image: ResourceArc<vk::Image>,
// }
//
// struct PendingDescriptorSetWrite {
//
// }







impl Drop for ResourceManager {
    fn drop(&mut self) {
        unsafe {
            println!("Cleaning up resource manager");
            self.dump_stats();

            // Wipe out any loaded assets. This will potentially drop ref counts on resources
            self.loaded_assets.destroy();

            // Drop all descriptors
            self.registered_descriptor_sets.destroy();

            // Now drop all resources with a zero ref count and warn for any resources that remain
            self.resources.destroy();

            println!("Dropping resource manager");
            self.dump_stats();
        }
    }
}

// We have to create pipelines when pipeline assets load and when swapchains are added/removed.
// Gathering all the info to hash and create a pipeline is a bit involved so we share the code
// here
#[derive(Clone)]
struct PipelineCreateData {
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_load_handles: Vec<LoadHandle>,
    shader_module_arcs: Vec<ResourceArc<vk::ShaderModule>>,
    shader_module_vk_objs: Vec<vk::ShaderModule>,

    descriptor_set_layout_arcs: Vec<ResourceArc<vk::DescriptorSetLayout>>,

    fixed_function_state: dsc::FixedFunctionState,

    pipeline_layout_def: dsc::PipelineLayout,
    pipeline_layout: ResourceArc<vk::PipelineLayout>,

    renderpass: dsc::RenderPass,
}

impl PipelineCreateData {
    pub fn new(
        resource_manager: &mut ResourceManager,
        pipeline_asset: PipelineAsset2,
        material_pass: &MaterialPass
    ) -> VkResult<Self> {
        //
        // Shader module metadata (required to create the pipeline key)
        //
        let mut shader_module_metas =
            Vec::with_capacity(material_pass.shaders.len());
        let mut shader_module_load_handles =
            Vec::with_capacity(material_pass.shaders.len());
        for stage in &material_pass.shaders {
            let shader_module_meta = dsc::ShaderModuleMeta {
                stage: stage.stage,
                entry_name: stage.entry_name.clone(),
            };
            shader_module_metas.push(shader_module_meta);
            shader_module_load_handles.push(stage.shader_module.load_handle());
        }

        //
        // Actual shader module resources (to create the pipeline)
        //
        let mut shader_module_arcs =
            Vec::with_capacity(material_pass.shaders.len());
        let mut shader_module_vk_objs =
            Vec::with_capacity(material_pass.shaders.len());
        for stage in &material_pass.shaders {
            let shader_module = resource_manager
                .loaded_assets
                .shader_modules
                .get_latest(stage.shader_module.load_handle())
                .unwrap();
            shader_module_arcs.push(shader_module.shader_module.clone());
            shader_module_vk_objs.push(shader_module.shader_module.get_raw());
        }

        //
        // Descriptor set layout
        //
        let mut descriptor_set_layout_arcs =
            Vec::with_capacity(material_pass.shader_interface.descriptor_set_layouts.len());
        let mut descriptor_set_layout_defs = Vec::with_capacity(material_pass.shader_interface.descriptor_set_layouts.len());
        for descriptor_set_layout_def in &material_pass.shader_interface.descriptor_set_layouts {
            let descriptor_set_layout_def = descriptor_set_layout_def.into();
            let descriptor_set_layout =
                resource_manager.resources.get_or_create_descriptor_set_layout(&descriptor_set_layout_def)?;
            descriptor_set_layout_arcs.push(descriptor_set_layout);
            descriptor_set_layout_defs.push(descriptor_set_layout_def);
        }

        //
        // Pipeline layout
        //
        let pipeline_layout_def = dsc::PipelineLayout {
            descriptor_set_layouts: descriptor_set_layout_defs,
            push_constant_ranges: material_pass.shader_interface.push_constant_ranges.clone(),
        };

        let pipeline_layout =
            resource_manager.resources.get_or_create_pipeline_layout(&pipeline_layout_def)?;

        let fixed_function_state = dsc::FixedFunctionState {
            vertex_input_state: material_pass.shader_interface.vertex_input_state.clone(),
            input_assembly_state: pipeline_asset.input_assembly_state,
            viewport_state: pipeline_asset.viewport_state,
            rasterization_state: pipeline_asset.rasterization_state,
            multisample_state: pipeline_asset.multisample_state,
            color_blend_state: pipeline_asset.color_blend_state,
            dynamic_state: pipeline_asset.dynamic_state,
        };

        Ok(PipelineCreateData {
            shader_module_metas,
            shader_module_load_handles,
            shader_module_arcs,
            shader_module_vk_objs,
            descriptor_set_layout_arcs,
            fixed_function_state,
            pipeline_layout_def,
            pipeline_layout,
            renderpass: pipeline_asset.renderpass,
        })
    }
}
