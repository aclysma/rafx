use crossbeam_channel::{Sender, Receiver};
use std::fmt::Formatter;
use std::hash::Hash;
use std::sync::{Weak, Arc};
use renderer_shell_vulkan::{VkResource, VkResourceDropSink, VkDeviceContext, VkImageRaw, VkImage};
use fnv::FnvHashMap;
use std::marker::PhantomData;
use ash::vk;
use ash::prelude::VkResult;
use crate::pipeline_description::SwapchainSurfaceInfo;
use super::PipelineCreateData;
use std::mem::ManuallyDrop;
use std::borrow::Borrow;
use crate::pipeline_description as dsc;
use atelier_assets::loader::LoadHandle;

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ResourceHash(u64);

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
    ResourceT: Copy,
{
    resource: ResourceT,
    resource_hash: ResourceHash,
}

impl<ResourceT> std::fmt::Debug for ResourceWithHash<ResourceT>
where
    ResourceT: std::fmt::Debug + Copy,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ResourceWithHash")
            .field("resource", &self.resource)
            .field("resource_hash", &self.resource_hash)
            .finish()
    }
}

struct ResourceArcInner<ResourceT>
where
    ResourceT: Copy,
{
    resource: ResourceWithHash<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
}

impl<ResourceT> Drop for ResourceArcInner<ResourceT>
where
    ResourceT: Copy,
{
    fn drop(&mut self) {
        self.drop_tx.send(self.resource.clone());
    }
}

impl<ResourceT> std::fmt::Debug for ResourceArcInner<ResourceT>
where
    ResourceT: std::fmt::Debug + Copy,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ResourceArcInner")
            .field("resource", &self.resource)
            .finish()
    }
}

#[derive(Clone)]
pub struct WeakResourceArc<ResourceT>
where
    ResourceT: Copy,
{
    inner: Weak<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> WeakResourceArc<ResourceT>
where
    ResourceT: Copy,
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
    ResourceT: std::fmt::Debug + Copy,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("WeakResourceArc")
            .field("inner", &self.inner)
            .finish()
    }
}

#[derive(Clone)]
pub struct ResourceArc<ResourceT>
where
    ResourceT: Copy,
{
    inner: Arc<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> ResourceArc<ResourceT>
where
    ResourceT: Copy,
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
    ResourceT: std::fmt::Debug + Copy,
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("ResourceArc")
            .field("inner", &self.inner)
            .finish()
    }
}

//
// A lookup of resources. They reference count using Arcs internally and send a signal when they
// drop. This allows the resources to be collected and disposed of
//
pub struct ResourceLookup<KeyT, ResourceT>
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
            {
                if upgrade.is_some() {
                    debug_assert!(self.keys.get(&hash).unwrap() == key);
                }
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

        log::trace!(
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
            log::trace!(
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
pub struct RenderPassKey {
    dsc: dsc::RenderPass,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineKey {
    pipeline_layout: dsc::PipelineLayout,
    renderpass: dsc::RenderPass,
    fixed_function_state: dsc::FixedFunctionState,
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_load_handles: Vec<LoadHandle>,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageKey {
    load_handle: LoadHandle,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageViewKey {
    image_load_handle: LoadHandle,
    image_view_meta: dsc::ImageViewMeta,
}

#[derive(Debug)]
pub struct ResourceMetrics {
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
pub struct ResourceLookupSet {
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
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> Self {
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

    pub fn on_frame_complete(&mut self) {
        self.shader_modules.on_frame_complete(&self.device_context);
        self.descriptor_set_layouts
            .on_frame_complete(&self.device_context);
        self.pipeline_layouts
            .on_frame_complete(&self.device_context);
        self.render_passes.on_frame_complete(&self.device_context);
        self.graphics_pipelines
            .on_frame_complete(&self.device_context);
        self.images.on_frame_complete(&self.device_context);
        self.image_views.on_frame_complete(&self.device_context);
        self.samplers.on_frame_complete(&self.device_context);
    }

    pub fn destroy(&mut self) {
        self.shader_modules.destroy(&self.device_context);
        self.descriptor_set_layouts.destroy(&self.device_context);
        self.pipeline_layouts.destroy(&self.device_context);
        self.render_passes.destroy(&self.device_context);
        self.graphics_pipelines.destroy(&self.device_context);
        self.images.destroy(&self.device_context);
        self.image_views.destroy(&self.device_context);
        self.samplers.destroy(&self.device_context);
    }

    pub fn metrics(&self) -> ResourceMetrics {
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

    pub fn get_or_create_shader_module(
        &mut self,
        shader_module: &dsc::ShaderModule,
    ) -> VkResult<ResourceArc<vk::ShaderModule>> {
        let hash = ResourceHash::from_key(shader_module);
        if let Some(shader_module) = self.shader_modules.get(hash, shader_module) {
            Ok(shader_module)
        } else {
            log::trace!(
                "Creating shader module\n[bytes: {}]",
                shader_module.code.len()
            );
            let resource = crate::pipeline_description::create_shader_module(
                self.device_context.device(),
                shader_module,
            )?;
            log::trace!("Created shader module {:?}", resource);
            let shader_module = self.shader_modules.insert(hash, shader_module, resource);
            Ok(shader_module)
        }
    }

    pub fn get_or_create_descriptor_set_layout(
        &mut self,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<ResourceArc<vk::DescriptorSetLayout>> {
        let hash = ResourceHash::from_key(descriptor_set_layout);
        if let Some(descriptor_set_layout) =
            self.descriptor_set_layouts.get(hash, descriptor_set_layout)
        {
            Ok(descriptor_set_layout)
        } else {
            log::trace!(
                "Creating descriptor set layout\n{:#?}",
                descriptor_set_layout
            );
            let resource = crate::pipeline_description::create_descriptor_set_layout(
                self.device_context.device(),
                descriptor_set_layout,
            )?;
            log::trace!("Created descriptor set layout {:?}", resource);
            let descriptor_set_layout =
                self.descriptor_set_layouts
                    .insert(hash, descriptor_set_layout, resource);
            Ok(descriptor_set_layout)
        }
    }

    pub fn get_or_create_pipeline_layout(
        &mut self,
        pipeline_layout_def: &dsc::PipelineLayout,
    ) -> VkResult<ResourceArc<vk::PipelineLayout>> {
        let hash = ResourceHash::from_key(pipeline_layout_def);
        if let Some(pipeline_layout) = self.pipeline_layouts.get(hash, pipeline_layout_def) {
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

            log::trace!("Creating pipeline layout\n{:#?}", pipeline_layout_def);
            let resource = crate::pipeline_description::create_pipeline_layout(
                self.device_context.device(),
                pipeline_layout_def,
                &descriptor_set_layouts,
            )?;
            log::trace!("Created pipeline layout {:?}", resource);
            let pipeline_layout = self
                .pipeline_layouts
                .insert(hash, pipeline_layout_def, resource);

            Ok(pipeline_layout)
        }
    }

    pub fn get_or_create_renderpass(
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
            log::trace!("Creating renderpass\n{:#?}", renderpass_key);
            let resource = crate::pipeline_description::create_renderpass(
                self.device_context.device(),
                renderpass,
                &swapchain_surface_info,
            )?;
            log::trace!("Created renderpass {:?}", resource);

            let renderpass = self.render_passes.insert(hash, &renderpass_key, resource);
            Ok(renderpass)
        }
    }

    pub fn get_or_create_graphics_pipeline(
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

        let renderpass = self
            .get_or_create_renderpass(&pipeline_create_data.renderpass, swapchain_surface_info)?;

        let hash = ResourceHash::from_key(&pipeline_key);
        if let Some(pipeline) = self.graphics_pipelines.get(hash, &pipeline_key) {
            Ok((renderpass, pipeline))
        } else {
            log::trace!("Creating pipeline\n{:#?}", pipeline_key);
            let resource = crate::pipeline_description::create_graphics_pipeline(
                &self.device_context.device(),
                &pipeline_create_data.fixed_function_state,
                pipeline_create_data.pipeline_layout.get_raw(),
                renderpass.get_raw(),
                &pipeline_create_data.shader_module_metas,
                &pipeline_create_data.shader_module_vk_objs,
                swapchain_surface_info,
            )?;
            log::trace!("Created pipeline {:?}", resource);

            let pipeline = self
                .graphics_pipelines
                .insert(hash, &pipeline_key, resource);
            Ok((renderpass, pipeline))
        }
    }

    pub fn insert_image(
        &mut self,
        load_handle: LoadHandle,
        image: ManuallyDrop<VkImage>,
    ) -> ResourceArc<VkImageRaw> {
        let image_key = ImageKey { load_handle };

        let hash = ResourceHash::from_key(&image_key);
        let raw_image = ManuallyDrop::into_inner(image).take_raw().unwrap();
        self.images.insert(hash, &image_key, raw_image)
    }

    pub fn get_or_create_image_view(
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
                load_handle: image_load_handle,
            };
            let image_load_handle_hash = ResourceHash::from_key(&image_load_handle);
            let image = self.images.get(image_load_handle_hash, &image_key).unwrap();

            log::trace!("Creating image view\n{:#?}", image_view_key);
            let resource = crate::pipeline_description::create_image_view(
                &self.device_context.device(),
                image.get_raw().image,
                image_view_meta,
            )?;
            log::trace!("Created image view\n{:#?}", resource);

            let image_view = self.image_views.insert(hash, &image_view_key, resource);
            Ok(image_view)
        }
    }
}
