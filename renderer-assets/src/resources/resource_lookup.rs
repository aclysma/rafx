use crossbeam_channel::{Sender, Receiver};
use std::hash::Hash;
use renderer_shell_vulkan::{
    VkResource, VkResourceDropSink, VkDeviceContext, VkImageRaw, VkImage, VkBufferRaw, VkBuffer,
};
use fnv::FnvHashMap;
use std::marker::PhantomData;
use ash::vk;
use ash::prelude::VkResult;
use crate::vk_description::SwapchainSurfaceInfo;
use std::mem::ManuallyDrop;
use crate::vk_description as dsc;
use crate::resources::ResourceArc;
use crate::resources::resource_arc::{WeakResourceArc, ResourceWithHash, ResourceId};
use std::sync::Arc;
use renderer_nodes::RenderPhaseIndex;

// #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
// pub(super) struct ResourceId(pub(super) u64);

// impl From<ResourceHash> for ResourceId {
//     fn from(resource_hash: ResourceHash) -> Self {
//         ResourceId(resource_hash.0)
//     }
// }

//TODO: Separate the renderpass description from the asset format. Make the description specify
// everything.

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ResourceHash(u64);

impl ResourceHash {
    pub fn from_key<KeyT: Hash>(key: &KeyT) -> ResourceHash {
        use std::hash::Hasher;
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        ResourceHash(hasher.finish())
    }
}

impl From<ResourceId> for ResourceHash {
    fn from(resource_id: ResourceId) -> Self {
        ResourceHash(resource_id.0)
    }
}

impl Into<ResourceId> for ResourceHash {
    fn into(self) -> ResourceId {
        ResourceId(self.0)
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
    //TODO: Add support for "cancelling" dropping stuff. This would likely be a ring of hashmaps.
    // that gets cycled.
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
        _key: &KeyT,
    ) -> Option<ResourceArc<ResourceT>> {
        if let Some(resource) = self.resources.get(&hash) {
            let upgrade = resource.upgrade();

            #[cfg(debug_assertions)]
            {
                if upgrade.is_some() {
                    debug_assert!(self.keys.get(&hash).unwrap() == _key);
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
        _key: &KeyT,
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

        let arc = ResourceArc::new(resource, hash.into(), self.drop_tx.clone());
        let downgraded = arc.downgrade();
        let old = self.resources.insert(hash, downgraded);
        assert!(old.is_none());

        #[cfg(debug_assertions)]
        {
            self.keys.insert(hash, _key.clone());
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
            self.resources.remove(&dropped.resource_hash.into());

            #[cfg(debug_assertions)]
            {
                self.keys.remove(&dropped.resource_hash.into());
            }
        }
    }

    fn len(&self) -> usize {
        self.resources.len()
    }

    fn on_frame_complete(
        &mut self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        self.handle_dropped_resources();
        self.drop_sink.on_frame_complete(device_context)
    }

    fn destroy(
        &mut self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        self.handle_dropped_resources();

        if !self.resources.is_empty() {
            log::warn!(
                "{} resource count {} > 0, resources will leak",
                core::any::type_name::<ResourceT>(),
                self.resources.len()
            );
        }

        self.drop_sink.destroy(device_context)
    }
}

//
// Keys for each resource type. (Some keys are simple and use types from crate::pipeline_description
// and some are a combination of the definitions and runtime state. (For example, combining a
// renderpass with the swapchain surface it would be applied to)
//

//TODO: Should I Arc the dsc objects in these keys?

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShaderModuleKey {
    code_hash: dsc::ShaderModuleCodeHash,
}

//TODO: The hashing here should probably be on the description after it is populated with
// fields from swapchain surface info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPassKey {
    dsc: dsc::RenderPass,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}

impl RenderPassKey {
    pub fn renderpass_def(&self) -> &dsc::RenderPass {
        &self.dsc
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameBufferKey {
    renderpass: dsc::RenderPass,
    image_view_keys: Vec<ImageViewKey>,
    framebuffer_meta: dsc::FramebufferMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialPassKey {
    pipeline_layout: dsc::PipelineLayout,
    fixed_function_state: dsc::FixedFunctionState,
    shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    shader_module_keys: Vec<ShaderModuleKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineKey {
    material_pass_key: MaterialPassKey,
    renderpass_key: RenderPassKey,
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
    pub shader_module_count: usize,
    pub descriptor_set_layout_count: usize,
    pub pipeline_layout_count: usize,
    pub renderpass_count: usize,
    pub framebuffer_count: usize,
    pub material_passes: usize,
    pub graphics_pipeline_count: usize,
    pub image_count: usize,
    pub image_view_count: usize,
    pub sampler_count: usize,
    pub buffer_count: usize,
}

#[derive(Debug, Clone)]
pub struct ShaderModuleResource {
    pub shader_module_key: ShaderModuleKey,
    pub shader_module_def: Arc<dsc::ShaderModule>,
    pub shader_module: vk::ShaderModule,
}

impl VkResource for ShaderModuleResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.shader_module)
    }
}

#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutResource {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_set_layout_def: dsc::DescriptorSetLayout,
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
    pub pipeline_layout_def: dsc::PipelineLayout,
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
pub struct MaterialPassResource {
    pub material_pass_key: MaterialPassKey,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,
    pub shader_modules: Vec<ResourceArc<ShaderModuleResource>>,
    // This is just cached, shader_modules handles cleaning these up
    pub shader_module_vk_objs: Vec<vk::ShaderModule>,
}

impl VkResource for MaterialPassResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        // for pipeline in resource.pipelines {
        //     VkResource::destroy(device_context, pipeline)?;
        // }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GraphicsPipelineResource {
    pub pipelines: Vec<vk::Pipeline>,
    pub pipeline_layout: ResourceArc<PipelineLayoutResource>,

    // Renderpasses must be re-registered regularly to the GraphicsPipelineCache. Otherwise, we
    // would have a cyclical reference between cached pipelines and their renderpasses.
    pub renderpass: ResourceArc<RenderPassResource>,
    // This does not have a ResourceArc<MaterialPassResource>. If we end up adding it here,
    // this will potentially cause GraphicsPipelineCache's strong ref to cached pipelines to keep
    // material pass resources alive.
}

impl VkResource for GraphicsPipelineResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        for pipeline in resource.pipelines {
            VkResource::destroy(device_context, pipeline)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RenderPassResource {
    pub renderpass: vk::RenderPass,
    pub renderpass_key: RenderPassKey,
}

impl VkResource for RenderPassResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.renderpass)
    }
}

#[derive(Debug, Clone)]
pub struct FramebufferResource {
    pub framebuffer: vk::Framebuffer,
    pub framebuffer_key: FrameBufferKey,
    pub renderpass: ResourceArc<RenderPassResource>,
    pub attachments: Vec<ResourceArc<ImageViewResource>>,
}

impl VkResource for FramebufferResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.framebuffer)
    }
}

#[derive(Debug, Clone)]
pub struct ImageResource {
    pub image: VkImageRaw,
    // Dynamic resources have no key
    pub image_key: Option<ImageKey>
}

impl VkResource for ImageResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.image)
    }
}

#[derive(Debug, Clone)]
pub struct ImageViewResource {
    pub image_view: vk::ImageView,
    pub image: ResourceArc<ImageResource>,
    // Dynamic resources have no key
    pub image_view_key: Option<ImageViewKey>,
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
    device_context: VkDeviceContext,

    shader_modules: ResourceLookup<ShaderModuleKey, ShaderModuleResource>,
    descriptor_set_layouts: ResourceLookup<dsc::DescriptorSetLayout, DescriptorSetLayoutResource>,
    pipeline_layouts: ResourceLookup<dsc::PipelineLayout, PipelineLayoutResource>,
    render_passes: ResourceLookup<RenderPassKey, RenderPassResource>,
    framebuffers: ResourceLookup<FrameBufferKey, FramebufferResource>,
    material_passes: ResourceLookup<MaterialPassKey, MaterialPassResource>,
    graphics_pipelines: ResourceLookup<GraphicsPipelineKey, GraphicsPipelineResource>,
    images: ResourceLookup<ImageKey, ImageResource>,
    image_views: ResourceLookup<ImageViewKey, ImageViewResource>,
    samplers: ResourceLookup<dsc::Sampler, vk::Sampler>,
    buffers: ResourceLookup<BufferKey, VkBufferRaw>,

    // Used to generate keys for images/buffers
    next_image_id: u64,
    next_buffer_id: u64,
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
            framebuffers: ResourceLookup::new(max_frames_in_flight),
            material_passes: ResourceLookup::new(max_frames_in_flight),
            graphics_pipelines: ResourceLookup::new(max_frames_in_flight),
            images: ResourceLookup::new(max_frames_in_flight),
            image_views: ResourceLookup::new(max_frames_in_flight),
            samplers: ResourceLookup::new(max_frames_in_flight),
            buffers: ResourceLookup::new(max_frames_in_flight),
            next_image_id: 0,
            next_buffer_id: 0,
        }
    }

    pub fn on_frame_complete(&mut self) -> VkResult<()> {
        self.images.on_frame_complete(&self.device_context)?;
        self.image_views.on_frame_complete(&self.device_context)?;
        self.buffers.on_frame_complete(&self.device_context)?;
        self.shader_modules
            .on_frame_complete(&self.device_context)?;
        self.samplers.on_frame_complete(&self.device_context)?;
        self.descriptor_set_layouts
            .on_frame_complete(&self.device_context)?;
        self.pipeline_layouts
            .on_frame_complete(&self.device_context)?;
        self.render_passes.on_frame_complete(&self.device_context)?;
        self.framebuffers.on_frame_complete(&self.device_context)?;
        self.material_passes
            .on_frame_complete(&self.device_context)?;
        self.graphics_pipelines
            .on_frame_complete(&self.device_context)?;
        Ok(())
    }

    pub fn destroy(&mut self) -> VkResult<()> {
        //WARNING: These need to be in order of dependencies to avoid frame-delays on destroying
        // resources.
        self.graphics_pipelines.destroy(&self.device_context)?;
        self.material_passes.destroy(&self.device_context)?;
        self.framebuffers.destroy(&self.device_context)?;
        self.render_passes.destroy(&self.device_context)?;
        self.pipeline_layouts.destroy(&self.device_context)?;
        self.descriptor_set_layouts.destroy(&self.device_context)?;
        self.samplers.destroy(&self.device_context)?;
        self.shader_modules.destroy(&self.device_context)?;
        self.buffers.destroy(&self.device_context)?;
        self.image_views.destroy(&self.device_context)?;
        self.images.destroy(&self.device_context)?;
        Ok(())
    }

    pub fn metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            shader_module_count: self.shader_modules.len(),
            descriptor_set_layout_count: self.descriptor_set_layouts.len(),
            pipeline_layout_count: self.pipeline_layouts.len(),
            renderpass_count: self.render_passes.len(),
            framebuffer_count: self.framebuffers.len(),
            material_passes: self.material_passes.len(),
            graphics_pipeline_count: self.graphics_pipelines.len(),
            image_count: self.images.len(),
            image_view_count: self.image_views.len(),
            sampler_count: self.samplers.len(),
            buffer_count: self.buffers.len(),
        }
    }

    pub fn get_or_create_shader_module(
        &mut self,
        shader_module_def: &Arc<dsc::ShaderModule>,
    ) -> VkResult<ResourceArc<ShaderModuleResource>> {
        let shader_module_key = ShaderModuleKey {
            code_hash: shader_module_def.code_hash,
        };

        let hash = ResourceHash::from_key(&shader_module_key);
        if let Some(shader_module) = self.shader_modules.get(hash, &shader_module_key) {
            Ok(shader_module)
        } else {
            log::trace!(
                "Creating shader module\n[hash: {:?} bytes: {}]",
                shader_module_key.code_hash,
                shader_module_def.code.len()
            );
            let shader_module =
                dsc::create_shader_module(self.device_context.device(), &*shader_module_def)?;
            let resource = ShaderModuleResource {
                shader_module,
                shader_module_def: shader_module_def.clone(),
                shader_module_key: shader_module_key.clone(),
            };
            log::trace!("Created shader module {:?}", resource);
            let shader_module = self
                .shader_modules
                .insert(hash, &shader_module_key, resource);
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

            let resource = dsc::create_sampler(self.device_context.device(), sampler)?;

            log::trace!("Created sampler {:?}", resource);
            let sampler = self.samplers.insert(hash, sampler, resource);
            Ok(sampler)
        }
    }

    pub fn get_or_create_descriptor_set_layout(
        &mut self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
    ) -> VkResult<ResourceArc<DescriptorSetLayoutResource>> {
        let hash = ResourceHash::from_key(descriptor_set_layout_def);
        if let Some(descriptor_set_layout) = self
            .descriptor_set_layouts
            .get(hash, descriptor_set_layout_def)
        {
            Ok(descriptor_set_layout)
        } else {
            log::trace!(
                "Creating descriptor set layout\n{:#?}",
                descriptor_set_layout_def
            );

            // Put all samplers into a hashmap so that we avoid collecting duplicates. This prevents
            // samplers from dropping out of scope and being destroyed
            let mut immutable_sampler_arcs = FnvHashMap::default();

            // But we also need to put raw vk objects into a format compatible with
            // create_descriptor_set_layout
            let mut immutable_sampler_vk_objs = Vec::with_capacity(
                descriptor_set_layout_def
                    .descriptor_set_layout_bindings
                    .len(),
            );

            // Get or create samplers and add them to the two above structures
            for x in &descriptor_set_layout_def.descriptor_set_layout_bindings {
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
            let resource = dsc::create_descriptor_set_layout(
                self.device_context.device(),
                descriptor_set_layout_def,
                &immutable_sampler_vk_objs,
            )?;

            // Flatten the hashmap into just the values
            let immutable_samplers = immutable_sampler_arcs.drain().map(|(_, x)| x).collect();

            // Create the resource object, which contains the descriptor set layout we created plus
            // ResourceArcs to the samplers, which must remain alive for the lifetime of the descriptor set
            let resource = DescriptorSetLayoutResource {
                descriptor_set_layout: resource,
                descriptor_set_layout_def: descriptor_set_layout_def.clone(),
                immutable_samplers,
            };

            log::trace!("Created descriptor set layout {:?}", resource);
            let descriptor_set_layout =
                self.descriptor_set_layouts
                    .insert(hash, descriptor_set_layout_def, resource);
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
            let resource = dsc::create_pipeline_layout(
                self.device_context.device(),
                pipeline_layout_def,
                &descriptor_set_layouts,
            )?;

            let resource = PipelineLayoutResource {
                pipeline_layout: resource,
                pipeline_layout_def: pipeline_layout_def.clone(),
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
    ) -> VkResult<ResourceArc<RenderPassResource>> {
        let renderpass_key = RenderPassKey {
            dsc: renderpass.clone(),
            swapchain_surface_info: swapchain_surface_info.clone(),
        };

        let hash = ResourceHash::from_key(&renderpass_key);
        if let Some(renderpass) = self.render_passes.get(hash, &renderpass_key) {
            Ok(renderpass)
        } else {
            log::trace!("Creating renderpass\n{:#?}", renderpass_key);
            let resource = dsc::create_renderpass(
                self.device_context.device(),
                renderpass,
                &swapchain_surface_info,
            )?;

            let resource = RenderPassResource {
                renderpass: resource,
                renderpass_key: renderpass_key.clone(),
            };

            log::trace!("Created renderpass {:?}", resource);

            let renderpass = self.render_passes.insert(hash, &renderpass_key, resource);
            Ok(renderpass)
        }
    }

    pub fn get_or_create_framebuffer(
        &mut self,
        renderpass: ResourceArc<RenderPassResource>,
        attachments: &[ResourceArc<ImageViewResource>],
        framebuffer_meta: &dsc::FramebufferMeta,
    ) -> VkResult<ResourceArc<FramebufferResource>> {
        let framebuffer_key = FrameBufferKey {
            renderpass: renderpass.get_raw().renderpass_key.dsc,
            image_view_keys: attachments
                .iter()
                .map(|resource| {
                    resource
                        .get_raw()
                        .image_view_key
                        .expect("Only keyed image views allowed in get_or_create_framebuffer")
                })
                .collect(),
            framebuffer_meta: framebuffer_meta.clone(),
        };

        let hash = ResourceHash::from_key(&framebuffer_key);
        if let Some(framebuffer) = self.framebuffers.get(hash, &framebuffer_key) {
            Ok(framebuffer)
        } else {
            log::trace!("Creating framebuffer\n{:#?}", framebuffer_key);

            let attachment_image_views: Vec<_> = attachments
                .iter()
                .map(|resource| resource.get_raw().image_view)
                .collect();

            let resource = dsc::create_framebuffer(
                self.device_context.device(),
                renderpass.get_raw().renderpass,
                &attachment_image_views,
                framebuffer_meta,
            )?;

            let resource = FramebufferResource {
                framebuffer: resource,
                framebuffer_key: framebuffer_key.clone(),
                renderpass,
                attachments: attachments.into(),
            };

            log::trace!("Created framebuffer {:?}", resource);

            let framebuffer = self.framebuffers.insert(hash, &framebuffer_key, resource);
            Ok(framebuffer)
        }
    }

    // Maybe we have a dedicated allocator for framebuffers and images that end up bound to framebuffers
    // These images shouldn't be throw-away because then we have to remake framebuffers constantly
    // So they either need to be inserted here or pooled in some way
    // pub fn get_or_create_framebuffer(
    //     &mut self,
    //     framebuffer: &dsc::FrameBufferMeta,
    //     images:
    // )

    pub fn get_or_create_material_pass(
        &mut self,
        shader_modules: Vec<ResourceArc<ShaderModuleResource>>,
        shader_module_metas: Vec<dsc::ShaderModuleMeta>,
        pipeline_layout: ResourceArc<PipelineLayoutResource>,
        fixed_function_state: dsc::FixedFunctionState,
    ) -> VkResult<ResourceArc<MaterialPassResource>> {
        let shader_module_keys = shader_modules
            .iter()
            .map(|x| x.get_raw().shader_module_key)
            .collect();
        let material_pass_key = MaterialPassKey {
            shader_module_metas,
            shader_module_keys,
            pipeline_layout: pipeline_layout.get_raw().pipeline_layout_def.clone(),
            fixed_function_state,
        };

        let hash = ResourceHash::from_key(&material_pass_key);
        if let Some(material_pass) = self.material_passes.get(hash, &material_pass_key) {
            Ok(material_pass)
        } else {
            log::trace!("Creating material pass\n{:#?}", material_pass_key);

            let shader_module_vk_objs = shader_modules
                .iter()
                .map(|x| x.get_raw().shader_module)
                .collect();

            let resource = MaterialPassResource {
                material_pass_key: material_pass_key.clone(),
                pipeline_layout,
                shader_modules,
                shader_module_vk_objs,
            };

            let material_pass = self
                .material_passes
                .insert(hash, &material_pass_key, resource);
            Ok(material_pass)
        }
    }

    pub fn get_or_create_graphics_pipeline(
        &mut self,
        material_pass: &ResourceArc<MaterialPassResource>,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> VkResult<ResourceArc<GraphicsPipelineResource>> {
        let pipeline_key = GraphicsPipelineKey {
            material_pass_key: material_pass.get_raw().material_pass_key.clone(),
            renderpass_key: renderpass.get_raw().renderpass_key.clone(),
        };

        let hash = ResourceHash::from_key(&pipeline_key);
        if let Some(pipeline) = self.graphics_pipelines.get(hash, &pipeline_key) {
            Ok(pipeline)
        } else {
            log::trace!("Creating pipeline\n{:#?}", pipeline_key);
            let pipelines = dsc::create_graphics_pipelines(
                &self.device_context.device(),
                &material_pass
                    .get_raw()
                    .material_pass_key
                    .fixed_function_state,
                material_pass
                    .get_raw()
                    .pipeline_layout
                    .get_raw()
                    .pipeline_layout,
                renderpass.get_raw().renderpass,
                &renderpass.get_raw().renderpass_key.dsc,
                &pipeline_key.material_pass_key.shader_module_metas,
                &material_pass.get_raw().shader_module_vk_objs,
                &pipeline_key.renderpass_key.swapchain_surface_info,
            )?;
            log::trace!("Created pipelines {:?}", pipelines);

            let resource = GraphicsPipelineResource {
                pipelines,
                pipeline_layout: material_pass.get_raw().pipeline_layout.clone(),
                renderpass: renderpass.clone(),
            };

            let pipeline = self
                .graphics_pipelines
                .insert(hash, &pipeline_key, resource);
            Ok(pipeline)
        }
    }

    // A key difference between this insert_image and the insert_image in a DynResourceAllocator
    // is that these can be retrieved. However, a mutable reference is required. This one is
    // more appropriate to use with loaded assets, and DynResourceAllocator with runtime assets
    pub fn insert_image(
        &mut self,
        image: ManuallyDrop<VkImage>,
    ) -> ResourceArc<ImageResource> {
        let raw_image = ManuallyDrop::into_inner(image).take_raw().unwrap();
        self.insert_raw_image(raw_image)
    }

    // This is useful for inserting swapchain images
    pub fn insert_raw_image(
        &mut self,
        raw_image: VkImageRaw,
    ) -> ResourceArc<ImageResource> {
        let image_key = ImageKey {
            id: self.next_image_id,
        };
        self.next_image_id += 1;

        let hash = ResourceHash::from_key(&image_key);

        let resource = ImageResource {
            image: raw_image,
            image_key: Some(image_key)
        };

        self.images.insert(hash, &image_key, resource)
    }

    //TODO: Support direct removal of raw images with verification that no references remain

    // A key difference between this insert_buffer and the insert_buffer in a DynResourceAllocator
    // is that these can be retrieved. However, a mutable reference is required. This one is
    // more appropriate to use with loaded assets, and DynResourceAllocator with runtime assets
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
        image: &ResourceArc<ImageResource>,
        image_view_meta: &dsc::ImageViewMeta,
    ) -> VkResult<ResourceArc<ImageViewResource>> {
        if image.get_raw().image_key.is_none() {
            log::error!("Tried to create an image view resource with a dynamic image");
            return Err(vk::Result::ERROR_UNKNOWN);
        }

        let image_view_key = ImageViewKey {
            image_key: image.get_raw().image_key.unwrap(),
            image_view_meta: image_view_meta.clone(),
        };

        let hash = ResourceHash::from_key(&image_view_key);
        if let Some(image_view) = self.image_views.get(hash, &image_view_key) {
            Ok(image_view)
        } else {
            log::trace!("Creating image view\n{:#?}", image_view_key);
            let resource = dsc::create_image_view(
                &self.device_context.device(),
                image.get_raw().image.image,
                image_view_meta,
            )?;
            log::trace!("Created image view\n{:#?}", resource);

            let resource = ImageViewResource {
                image_view: resource,
                image_view_key: Some(image_view_key.clone()),
                image: image.clone(),
            };

            let image_view = self.image_views.insert(hash, &image_view_key, resource);
            Ok(image_view)
        }
    }

    // pub fn get_or_create_frame_buffer(
    //     &mut self,
    //     frame_buffer_meta: dsc::FrameBufferMeta,
    //     images: dsc::ImageViewMeta,
    // )
}
