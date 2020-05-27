use crossbeam_channel::{Sender, Receiver};
use std::fmt::Formatter;
use std::hash::Hash;
use std::sync::{Weak, Arc};
use renderer_shell_vulkan::{
    VkResource, VkResourceDropSink, VkDeviceContext, VkImageRaw, VkImage, VkBufferRaw, VkBuffer,
};
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
    ResourceT: Clone,
{
    resource: ResourceT,
    resource_hash: ResourceHash,
}

impl<ResourceT> std::fmt::Debug for ResourceWithHash<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
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
    ResourceT: Clone,
{
    resource: ResourceWithHash<ResourceT>,
    drop_tx: Sender<ResourceWithHash<ResourceT>>,
}

impl<ResourceT> Drop for ResourceArcInner<ResourceT>
where
    ResourceT: Clone,
{
    fn drop(&mut self) {
        self.drop_tx.send(self.resource.clone());
    }
}

impl<ResourceT> std::fmt::Debug for ResourceArcInner<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
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
    ResourceT: Clone,
{
    inner: Weak<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> WeakResourceArc<ResourceT>
where
    ResourceT: Clone,
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
    ResourceT: std::fmt::Debug + Clone,
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
    ResourceT: Clone,
{
    inner: Arc<ResourceArcInner<ResourceT>>,
}

impl<ResourceT> ResourceArc<ResourceT>
where
    ResourceT: Clone,
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
        self.inner.resource.borrow().resource.clone()
    }

    pub fn downgrade(&self) -> WeakResourceArc<ResourceT> {
        let inner = Arc::downgrade(&self.inner);
        WeakResourceArc { inner }
    }
}

impl<ResourceT> std::fmt::Debug for ResourceArc<ResourceT>
where
    ResourceT: std::fmt::Debug + Clone,
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
    ResourceT: VkResource + Clone,
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
    ResourceT: VkResource + Clone + std::fmt::Debug,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ImageKey {
    id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BufferKey {
    id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageViewKey {
    image_key: ImageKey,
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
    buffer_count: usize,
}

#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutResource {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub immutable_samplers: Vec<ResourceArc<vk::Sampler>>,
}

impl VkResource for DescriptorSetLayoutResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.descriptor_set_layout)
    }
}

#[derive(Debug, Clone)]
pub struct PipelineLayoutResource {
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_sets: Vec<ResourceArc<DescriptorSetLayoutResource>>,
}

impl VkResource for PipelineLayoutResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.pipeline_layout)
    }
}

#[derive(Debug, Clone)]
pub struct PipelineResource {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,
    pub renderpass: ResourceArc<vk::RenderPass>,
}

impl VkResource for PipelineResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.pipeline)
    }
}

#[derive(Debug, Clone)]
pub struct ImageViewResource {
    pub image_view: vk::ImageView,
    pub image: ResourceArc<VkImageRaw>,
}

impl VkResource for ImageViewResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.image_view)
    }
}

//
// Handles raw lookup and destruction of GPU resources. Everything is reference counted. No safety
// is provided for dependencies/order of destruction. The general expectation is that anything
// dropped can safely be destroyed after a few frames have passed (based on max number of frames
// that can be submitted to the GPU)
//
//TODO: Some of the resources like buffers and images don't need to be "keyed" and could probably
// be kept in a slab. We *do* need a way to access and quickly remove elements though, and whatever
// key we use is sent through a Sender/Receiver pair to be dropped later.
pub struct ResourceLookupSet {
    pub device_context: VkDeviceContext,
    pub shader_modules: ResourceLookup<dsc::ShaderModule, vk::ShaderModule>,
    pub descriptor_set_layouts:
        ResourceLookup<dsc::DescriptorSetLayout, DescriptorSetLayoutResource>,
    pub pipeline_layouts: ResourceLookup<dsc::PipelineLayout, PipelineLayoutResource>,
    pub render_passes: ResourceLookup<RenderPassKey, vk::RenderPass>,
    pub graphics_pipelines: ResourceLookup<GraphicsPipelineKey, PipelineResource>,
    pub images: ResourceLookup<ImageKey, VkImageRaw>,
    pub image_views: ResourceLookup<ImageViewKey, ImageViewResource>,
    pub samplers: ResourceLookup<dsc::Sampler, vk::Sampler>,
    pub buffers: ResourceLookup<BufferKey, VkBufferRaw>,

    // Used to generate keys for images/buffers
    pub next_image_id: u64,
    pub next_buffer_id: u64,
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
            buffers: ResourceLookup::new(max_frames_in_flight),
            next_image_id: 0,
            next_buffer_id: 0,
        }
    }

    pub fn on_frame_complete(&mut self) {
        self.buffers.on_frame_complete(&self.device_context);
        self.shader_modules.on_frame_complete(&self.device_context);
        self.samplers.on_frame_complete(&self.device_context);
        self.descriptor_set_layouts
            .on_frame_complete(&self.device_context);
        self.pipeline_layouts
            .on_frame_complete(&self.device_context);
        self.render_passes.on_frame_complete(&self.device_context);
        self.graphics_pipelines
            .on_frame_complete(&self.device_context);
        self.images.on_frame_complete(&self.device_context);
        self.image_views.on_frame_complete(&self.device_context);
    }

    pub fn destroy(&mut self) {
        //WARNING: These need to be in order of dependencies to avoid frame-delays on destroying
        // resources.
        self.image_views.destroy(&self.device_context);
        self.images.destroy(&self.device_context);
        self.graphics_pipelines.destroy(&self.device_context);
        self.render_passes.destroy(&self.device_context);
        self.pipeline_layouts.destroy(&self.device_context);
        self.descriptor_set_layouts.destroy(&self.device_context);
        self.samplers.destroy(&self.device_context);
        self.shader_modules.destroy(&self.device_context);
        self.buffers.destroy(&self.device_context);
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
            buffer_count: self.buffers.len(),
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

    pub fn get_or_create_sampler(
        &mut self,
        sampler: &dsc::Sampler,
    ) -> VkResult<ResourceArc<vk::Sampler>> {
        let hash = ResourceHash::from_key(sampler);
        if let Some(sampler) = self.samplers.get(hash, sampler) {
            Ok(sampler)
        } else {
            log::trace!("Creating sampler\n{:#?}", sampler);

            let resource =
                crate::pipeline_description::create_sampler(self.device_context.device(), sampler)?;

            log::trace!("Created sampler {:?}", resource);
            let sampler = self.samplers.insert(hash, sampler, resource);
            Ok(sampler)
        }
    }

    pub fn get_or_create_descriptor_set_layout(
        &mut self,
        descriptor_set_layout: &dsc::DescriptorSetLayout,
    ) -> VkResult<ResourceArc<DescriptorSetLayoutResource>> {
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

            // Put all samplers into a hashmap so that we avoid collecting duplicates. This prevents
            // samplers from dropping out of scope and being destroyed
            let mut immutable_sampler_arcs = FnvHashMap::default();

            // But we also need to put raw vk objects into a format compatible with
            // create_descriptor_set_layout
            let mut immutable_sampler_vk_objs =
                Vec::with_capacity(descriptor_set_layout.descriptor_set_layout_bindings.len());

            // Get or create samplers and add them to the two above structures
            for x in &descriptor_set_layout.descriptor_set_layout_bindings {
                if let Some(sampler_defs) = &x.immutable_samplers {
                    let mut samplers = Vec::with_capacity(sampler_defs.len());
                    for sampler_def in sampler_defs {
                        let sampler = self.get_or_create_sampler(sampler_def)?;
                        samplers.push(sampler.get_raw());
                        immutable_sampler_arcs.insert(sampler_def, sampler);
                    }
                    immutable_sampler_vk_objs.push(Some(samplers));
                } else {
                    immutable_sampler_vk_objs.push(None);
                }
            }

            // Create the descriptor set layout
            let resource = crate::pipeline_description::create_descriptor_set_layout(
                self.device_context.device(),
                descriptor_set_layout,
                &immutable_sampler_vk_objs,
            )?;

            // Flatten the hashmap into just the values
            let immutable_samplers = immutable_sampler_arcs.drain().map(|(_, x)| x).collect();

            // Create the resource object, which contains the descriptor set layout we created plus
            // ResourceArcs to the samplers, which must remain alive for the lifetime of the descriptor set
            let resource = DescriptorSetLayoutResource {
                descriptor_set_layout: resource,
                immutable_samplers,
            };

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
    ) -> VkResult<ResourceArc<PipelineLayoutResource>> {
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
                descriptor_set_layouts
                    .push(loaded_descriptor_set_layout.get_raw().descriptor_set_layout);
            }

            log::trace!("Creating pipeline layout\n{:#?}", pipeline_layout_def);
            let resource = crate::pipeline_description::create_pipeline_layout(
                self.device_context.device(),
                pipeline_layout_def,
                &descriptor_set_layouts,
            )?;

            let resource = PipelineLayoutResource {
                pipeline_layout: resource,
                descriptor_sets: descriptor_set_layout_arcs,
            };

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
    ) -> VkResult<ResourceArc<PipelineResource>> {
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
            Ok(pipeline)
        } else {
            log::trace!("Creating pipeline\n{:#?}", pipeline_key);
            let resource = crate::pipeline_description::create_graphics_pipeline(
                &self.device_context.device(),
                &pipeline_create_data.fixed_function_state,
                pipeline_create_data
                    .pipeline_layout
                    .get_raw()
                    .pipeline_layout,
                renderpass.get_raw(),
                &pipeline_create_data.shader_module_metas,
                &pipeline_create_data.shader_module_vk_objs,
                swapchain_surface_info,
            )?;
            log::trace!("Created pipeline {:?}", resource);

            let resource = PipelineResource {
                pipeline: resource,
                pipeline_layout: pipeline_create_data.pipeline_layout.clone(),
                renderpass: renderpass.clone(),
            };

            let pipeline = self
                .graphics_pipelines
                .insert(hash, &pipeline_key, resource);
            Ok(pipeline)
        }
    }

    pub fn insert_image(
        &mut self,
        image: ManuallyDrop<VkImage>,
    ) -> (ImageKey, ResourceArc<VkImageRaw>) {
        let image_key = ImageKey {
            id: self.next_image_id,
        };
        self.next_image_id += 1;

        let hash = ResourceHash::from_key(&image_key);
        let raw_image = ManuallyDrop::into_inner(image).take_raw().unwrap();
        let image = self.images.insert(hash, &image_key, raw_image);
        (image_key, image)
    }

    pub fn insert_buffer(
        &mut self,
        buffer: ManuallyDrop<VkBuffer>,
    ) -> (BufferKey, ResourceArc<VkBufferRaw>) {
        let buffer_key = BufferKey {
            id: self.next_buffer_id,
        };
        self.next_buffer_id += 1;

        let hash = ResourceHash::from_key(&buffer_key);
        let raw_buffer = ManuallyDrop::into_inner(buffer).take_raw().unwrap();
        let buffer = self.buffers.insert(hash, &buffer_key, raw_buffer);
        (buffer_key, buffer)
    }

    pub fn get_or_create_image_view(
        &mut self,
        image_key: ImageKey,
        image_view_meta: &dsc::ImageViewMeta,
    ) -> VkResult<ResourceArc<ImageViewResource>> {
        let image_view_key = ImageViewKey {
            image_key,
            image_view_meta: image_view_meta.clone(),
        };

        let hash = ResourceHash::from_key(&image_view_key);
        if let Some(image_view) = self.image_views.get(hash, &image_view_key) {
            Ok(image_view)
        } else {
            // let image_key = ImageKey {
            //     image_key: image_load_handle,
            // };
            let image_key_hash = ResourceHash::from_key(&image_key);
            let image = self.images.get(image_key_hash, &image_key).unwrap();

            log::trace!("Creating image view\n{:#?}", image_view_key);
            let resource = crate::pipeline_description::create_image_view(
                &self.device_context.device(),
                image.get_raw().image,
                image_view_meta,
            )?;
            log::trace!("Created image view\n{:#?}", resource);

            let resource = ImageViewResource {
                image_view: resource,
                image: image.clone(),
            };

            let image_view = self.image_views.insert(hash, &image_view_key, resource);
            Ok(image_view)
        }
    }
}
