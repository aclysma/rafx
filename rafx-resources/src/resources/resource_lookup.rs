use crate::resources::resource_arc::{ResourceId, ResourceWithHash, WeakResourceArc};
use crate::resources::ResourceArc;
use crate::vk_description as dsc;
use crate::vk_description::{FramebufferMeta, SwapchainSurfaceInfo};
use ash::prelude::VkResult;
use ash::vk;
use bitflags::_core::sync::atomic::AtomicU64;
use crossbeam_channel::{Receiver, Sender};
use fnv::FnvHashMap;
use rafx_shell_vulkan::{
    VkBuffer, VkBufferRaw, VkDeviceContext, VkImage, VkImageRaw, VkResource, VkResourceDropSink,
};
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ResourceHash(u64);

impl ResourceHash {
    pub fn from_key<KeyT: Hash>(key: &KeyT) -> ResourceHash {
        use fnv::FnvHasher;
        use std::hash::Hasher;
        let mut hasher = FnvHasher::default();
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
pub struct ResourceLookupInner<KeyT, ResourceT>
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
    #[cfg(debug_assertions)]
    lock_call_count_previous_frame: u64,
    #[cfg(debug_assertions)]
    lock_call_count: u64,
    create_count_previous_frame: u64,
    create_count: u64,
}

//TODO: Don't love using a mutex here. If this becomes a performance bottleneck:
// - Try making locks more granular (something like dashmap)
// - Have a read-only hashmap that's checked first and then a read/write map that's checked if the
//   read-only fails. At a later sync point, copy new data from the read-write into the read. This
//   could occur during the extract phase. Or could potentially double-buffer the read-only map
//   and swap them.
pub struct ResourceLookup<KeyT, ResourceT>
where
    KeyT: Eq + Hash + Clone,
    ResourceT: VkResource + Clone,
{
    inner: Mutex<ResourceLookupInner<KeyT, ResourceT>>,
}

impl<KeyT, ResourceT> ResourceLookup<KeyT, ResourceT>
where
    KeyT: Eq + Hash + Clone,
    ResourceT: VkResource + Clone + std::fmt::Debug,
{
    fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        let inner = ResourceLookupInner {
            resources: Default::default(),
            drop_sink: VkResourceDropSink::new(max_frames_in_flight),
            drop_tx,
            drop_rx,
            phantom_data: Default::default(),
            #[cfg(debug_assertions)]
            keys: Default::default(),
            #[cfg(debug_assertions)]
            lock_call_count_previous_frame: 0,
            #[cfg(debug_assertions)]
            lock_call_count: 0,
            create_count_previous_frame: 0,
            create_count: 0,
        };

        ResourceLookup {
            inner: Mutex::new(inner),
        }
    }

    fn do_get(
        inner: &mut ResourceLookupInner<KeyT, ResourceT>,
        hash: ResourceHash,
        _key: &KeyT,
    ) -> Option<ResourceArc<ResourceT>> {
        let resource = inner.resources.get(&hash);

        if let Some(resource) = resource {
            let upgrade = resource.upgrade();
            #[cfg(debug_assertions)]
            if upgrade.is_some() {
                debug_assert!(inner.keys.get(&hash).unwrap() == _key);
            }

            upgrade
        } else {
            None
        }
    }

    fn do_create<F>(
        inner: &mut ResourceLookupInner<KeyT, ResourceT>,
        hash: ResourceHash,
        _key: &KeyT,
        create_resource_fn: F,
    ) -> VkResult<ResourceArc<ResourceT>>
    where
        F: FnOnce() -> VkResult<ResourceT>,
    {
        // Process any pending drops. If we don't do this, it's possible that the pending drop could
        // wipe out the state we're about to set
        Self::handle_dropped_resources(inner);

        inner.create_count += 1;

        let resource = (create_resource_fn)()?;
        log::trace!(
            "insert resource {} {:?}",
            core::any::type_name::<ResourceT>(),
            resource
        );

        let arc = ResourceArc::new(resource, hash.into(), inner.drop_tx.clone());
        let downgraded = arc.downgrade();
        let old = inner.resources.insert(hash, downgraded);
        assert!(old.is_none());

        #[cfg(debug_assertions)]
        {
            inner.keys.insert(hash, _key.clone());
            assert!(old.is_none());
        }

        Ok(arc)
    }

    fn get(
        &self,
        key: &KeyT,
    ) -> Option<ResourceArc<ResourceT>> {
        let hash = ResourceHash::from_key(key);
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count += 1;
        }

        Self::do_get(&mut *guard, hash, key)
    }

    fn create<F>(
        &self,
        key: &KeyT,
        create_resource_fn: F,
    ) -> VkResult<ResourceArc<ResourceT>>
    where
        F: FnOnce() -> VkResult<ResourceT>,
    {
        let hash = ResourceHash::from_key(key);
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count += 1;
        }

        Self::do_create(&mut *guard, hash, key, create_resource_fn)
    }

    fn get_or_create<F>(
        &self,
        key: &KeyT,
        create_resource_fn: F,
    ) -> VkResult<ResourceArc<ResourceT>>
    where
        F: FnOnce() -> VkResult<ResourceT>,
    {
        let hash = ResourceHash::from_key(key);

        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count += 1;
        }

        if let Some(resource) = Self::do_get(&mut *guard, hash, key) {
            //println!("get {} {:?}", core::any::type_name::<ResourceT>(), hash);
            Ok(resource)
        } else {
            //println!("create {} {:?}", core::any::type_name::<ResourceT>(), hash);
            Self::do_create(&mut *guard, hash, key, create_resource_fn)
        }
    }

    fn handle_dropped_resources(inner: &mut ResourceLookupInner<KeyT, ResourceT>) {
        for dropped in inner.drop_rx.try_iter() {
            log::trace!(
                "dropping {} {:?}",
                core::any::type_name::<ResourceT>(),
                dropped.resource
            );
            inner.drop_sink.retire(dropped.resource);
            inner.resources.remove(&dropped.resource_hash.into());

            #[cfg(debug_assertions)]
            {
                inner.keys.remove(&dropped.resource_hash.into());
            }
        }
    }

    fn on_frame_complete(
        &self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count_previous_frame = guard.lock_call_count + 1;
            guard.lock_call_count = 0;
        }

        guard.create_count_previous_frame = guard.create_count;
        guard.create_count = 0;

        Self::handle_dropped_resources(&mut guard);
        guard.drop_sink.on_frame_complete(device_context)
    }

    fn metrics(&self) -> ResourceLookupMetric {
        let guard = self.inner.lock().unwrap();
        ResourceLookupMetric {
            count: guard.resources.len(),
            previous_frame_create_count: guard.create_count_previous_frame,
            #[cfg(debug_assertions)]
            previous_frame_lock_call_count: guard.lock_call_count_previous_frame,
        }
    }

    fn destroy(
        &self,
        device_context: &VkDeviceContext,
    ) -> VkResult<()> {
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count += 1;
        }

        Self::handle_dropped_resources(&mut guard);

        if !guard.resources.is_empty() {
            log::warn!(
                "{} resource count {} > 0, resources will leak",
                core::any::type_name::<ResourceT>(),
                guard.resources.len()
            );
        }

        guard.drop_sink.destroy(device_context)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayoutKey {
    def: dsc::DescriptorSetLayout,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineLayoutKey {
    def: dsc::PipelineLayout,
}

//TODO: The hashing here should probably be on the description after it is populated with
// fields from swapchain surface info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPassKey {
    dsc: Arc<dsc::RenderPass>,
    swapchain_surface_info: dsc::SwapchainSurfaceInfo,
}

impl RenderPassKey {
    pub fn renderpass_def(&self) -> &dsc::RenderPass {
        &self.dsc
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameBufferKey {
    pub renderpass: Arc<dsc::RenderPass>,
    pub image_view_keys: Vec<ImageViewKey>,
    pub framebuffer_meta: dsc::FramebufferMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialPassVertexInput {
    pub semantic: String,
    pub location: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialPassKey {
    pub pipeline_layout: dsc::PipelineLayout,
    pub fixed_function_state: Arc<dsc::FixedFunctionState>,
    pub vertex_inputs: Arc<Vec<MaterialPassVertexInput>>,
    pub shader_module_metas: Vec<dsc::ShaderModuleMeta>,
    pub shader_module_keys: Vec<ShaderModuleKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineKey {
    material_pass_key: MaterialPassKey,
    renderpass_key: RenderPassKey,
    framebuffer_meta: FramebufferMeta,
    vertex_input_state: Arc<dsc::PipelineVertexInputState>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ImageKey {
    id: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BufferKey {
    id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SamplerKey {
    sampler_def: dsc::Sampler,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageViewKey {
    image_key: ImageKey,
    image_view_meta: dsc::ImageViewMeta,
}

#[derive(Debug)]
pub struct ResourceLookupMetric {
    pub count: usize,
    pub previous_frame_create_count: u64,
    #[cfg(debug_assertions)]
    pub previous_frame_lock_call_count: u64,
}

#[derive(Debug)]
pub struct ResourceMetrics {
    pub shader_module_metrics: ResourceLookupMetric,
    pub descriptor_set_layout_metrics: ResourceLookupMetric,
    pub pipeline_layout_metrics: ResourceLookupMetric,
    pub renderpass_metrics: ResourceLookupMetric,
    pub framebuffer_metrics: ResourceLookupMetric,
    pub material_pass_metrics: ResourceLookupMetric,
    pub graphics_pipeline_metrics: ResourceLookupMetric,
    pub image_metrics: ResourceLookupMetric,
    pub image_view_metrics: ResourceLookupMetric,
    pub sampler_metrics: ResourceLookupMetric,
    pub buffer_metrics: ResourceLookupMetric,
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
    pub immutable_samplers: Vec<ResourceArc<SamplerResource>>,
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
        _device_context: &VkDeviceContext,
        _resource: Self,
    ) -> VkResult<()> {
        // Nothing needs explicit destroying
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GraphicsPipelineResource {
    // These are per subpass of the renderpass this pipeline executes in
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
    pub image_key: Option<ImageKey>,
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
    pub image_view_meta: dsc::ImageViewMeta,
}

impl VkResource for ImageViewResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.image_view)
    }
}

#[derive(Debug, Clone)]
pub struct SamplerResource {
    pub sampler: vk::Sampler,
    pub sampler_key: SamplerKey,
}

impl VkResource for SamplerResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.sampler)
    }
}

#[derive(Debug, Clone)]
pub struct BufferResource {
    pub buffer: VkBufferRaw,
    // Dynamic resources have no key
    pub buffer_key: Option<BufferKey>,
}

impl VkResource for BufferResource {
    fn destroy(
        device_context: &VkDeviceContext,
        resource: Self,
    ) -> VkResult<()> {
        VkResource::destroy(device_context, resource.buffer)
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
pub struct ResourceLookupSetInner {
    device_context: VkDeviceContext,

    shader_modules: ResourceLookup<ShaderModuleKey, ShaderModuleResource>,
    descriptor_set_layouts: ResourceLookup<DescriptorSetLayoutKey, DescriptorSetLayoutResource>,
    pipeline_layouts: ResourceLookup<PipelineLayoutKey, PipelineLayoutResource>,
    render_passes: ResourceLookup<RenderPassKey, RenderPassResource>,
    framebuffers: ResourceLookup<FrameBufferKey, FramebufferResource>,
    material_passes: ResourceLookup<MaterialPassKey, MaterialPassResource>,
    graphics_pipelines: ResourceLookup<GraphicsPipelineKey, GraphicsPipelineResource>,
    images: ResourceLookup<ImageKey, ImageResource>,
    image_views: ResourceLookup<ImageViewKey, ImageViewResource>,
    samplers: ResourceLookup<SamplerKey, SamplerResource>,
    buffers: ResourceLookup<BufferKey, BufferResource>,

    // Used to generate keys for images/buffers
    next_image_id: AtomicU64,
    next_buffer_id: AtomicU64,
}

#[derive(Clone)]
pub struct ResourceLookupSet {
    inner: Arc<ResourceLookupSetInner>,
}

impl ResourceLookupSet {
    pub fn new(
        device_context: &VkDeviceContext,
        max_frames_in_flight: u32,
    ) -> Self {
        let set = ResourceLookupSetInner {
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
            next_image_id: AtomicU64::new(0),
            next_buffer_id: AtomicU64::new(0),
        };

        ResourceLookupSet {
            inner: Arc::new(set),
        }
    }

    #[profiling::function]
    pub fn on_frame_complete(&self) -> VkResult<()> {
        self.inner
            .images
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .image_views
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .buffers
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .shader_modules
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .samplers
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .descriptor_set_layouts
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .pipeline_layouts
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .render_passes
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .framebuffers
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .material_passes
            .on_frame_complete(&self.inner.device_context)?;
        self.inner
            .graphics_pipelines
            .on_frame_complete(&self.inner.device_context)?;
        Ok(())
    }

    pub fn destroy(&self) -> VkResult<()> {
        //WARNING: These need to be in order of dependencies to avoid frame-delays on destroying
        // resources.
        self.inner
            .graphics_pipelines
            .destroy(&self.inner.device_context)?;
        self.inner
            .material_passes
            .destroy(&self.inner.device_context)?;
        self.inner
            .framebuffers
            .destroy(&self.inner.device_context)?;
        self.inner
            .render_passes
            .destroy(&self.inner.device_context)?;
        self.inner
            .pipeline_layouts
            .destroy(&self.inner.device_context)?;
        self.inner
            .descriptor_set_layouts
            .destroy(&self.inner.device_context)?;
        self.inner.samplers.destroy(&self.inner.device_context)?;
        self.inner
            .shader_modules
            .destroy(&self.inner.device_context)?;
        self.inner.buffers.destroy(&self.inner.device_context)?;
        self.inner.image_views.destroy(&self.inner.device_context)?;
        self.inner.images.destroy(&self.inner.device_context)?;
        Ok(())
    }

    pub fn metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            shader_module_metrics: self.inner.shader_modules.metrics(),
            descriptor_set_layout_metrics: self.inner.descriptor_set_layouts.metrics(),
            pipeline_layout_metrics: self.inner.pipeline_layouts.metrics(),
            renderpass_metrics: self.inner.render_passes.metrics(),
            framebuffer_metrics: self.inner.framebuffers.metrics(),
            material_pass_metrics: self.inner.material_passes.metrics(),
            graphics_pipeline_metrics: self.inner.graphics_pipelines.metrics(),
            image_metrics: self.inner.images.metrics(),
            image_view_metrics: self.inner.image_views.metrics(),
            sampler_metrics: self.inner.samplers.metrics(),
            buffer_metrics: self.inner.buffers.metrics(),
        }
    }

    pub fn get_or_create_shader_module(
        &self,
        shader_module_def: &Arc<dsc::ShaderModule>,
    ) -> VkResult<ResourceArc<ShaderModuleResource>> {
        let shader_module_key = ShaderModuleKey {
            code_hash: shader_module_def.code_hash,
        };

        self.inner
            .shader_modules
            .get_or_create(&shader_module_key, || {
                log::trace!(
                    "Creating shader module\n[hash: {:?} bytes: {}]",
                    shader_module_key.code_hash,
                    shader_module_def.code.len()
                );
                let shader_module = dsc::create_shader_module(
                    self.inner.device_context.device(),
                    &*shader_module_def,
                )?;
                let resource = ShaderModuleResource {
                    shader_module,
                    shader_module_def: shader_module_def.clone(),
                    shader_module_key: shader_module_key.clone(),
                };
                log::trace!("Created shader module {:?}", resource);
                Ok(resource)
            })
    }

    pub fn get_or_create_sampler(
        &self,
        sampler: &dsc::Sampler,
    ) -> VkResult<ResourceArc<SamplerResource>> {
        let sampler_key = SamplerKey {
            sampler_def: sampler.clone(),
        };

        self.inner.samplers.get_or_create(&sampler_key, || {
            log::trace!("Creating sampler\n{:#?}", sampler);

            let resource = dsc::create_sampler(self.inner.device_context.device(), sampler)?;

            let resource = SamplerResource {
                sampler: resource,
                sampler_key: sampler_key.clone(),
            };

            log::trace!("Created sampler {:?}", resource);
            Ok(resource)
        })
    }

    pub fn get_or_create_descriptor_set_layout(
        &self,
        descriptor_set_layout_def: &dsc::DescriptorSetLayout,
    ) -> VkResult<ResourceArc<DescriptorSetLayoutResource>> {
        let key = DescriptorSetLayoutKey {
            def: descriptor_set_layout_def.clone(),
        };

        if let Some(descriptor_set_layout) = self.inner.descriptor_set_layouts.get(&key) {
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
                        samplers.push(sampler.get_raw().sampler);
                        immutable_sampler_arcs.insert(sampler_def, sampler);
                    }
                    immutable_sampler_vk_objs.push(Some(samplers));
                } else {
                    immutable_sampler_vk_objs.push(None);
                }
            }

            self.inner.descriptor_set_layouts.get_or_create(&key, || {
                // Create the descriptor set layout
                let resource = dsc::create_descriptor_set_layout(
                    self.inner.device_context.device(),
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
                Ok(resource)
            })
        }
    }

    pub fn get_or_create_pipeline_layout(
        &self,
        pipeline_layout_def: &dsc::PipelineLayout,
    ) -> VkResult<ResourceArc<PipelineLayoutResource>> {
        let key = PipelineLayoutKey {
            def: pipeline_layout_def.clone(),
        };

        if let Some(pipeline_layout) = self.inner.pipeline_layouts.get(&key) {
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

            self.inner.pipeline_layouts.get_or_create(&key, || {
                log::trace!("Creating pipeline layout\n{:#?}", pipeline_layout_def);
                let resource = dsc::create_pipeline_layout(
                    self.inner.device_context.device(),
                    pipeline_layout_def,
                    &descriptor_set_layouts,
                )?;

                let resource = PipelineLayoutResource {
                    pipeline_layout: resource,
                    pipeline_layout_def: pipeline_layout_def.clone(),
                    descriptor_sets: descriptor_set_layout_arcs,
                };

                log::trace!("Created pipeline layout {:?}", resource);
                Ok(resource)
            })
        }
    }

    pub fn get_or_create_renderpass(
        &self,
        renderpass: Arc<dsc::RenderPass>,
        swapchain_surface_info: &SwapchainSurfaceInfo,
    ) -> VkResult<ResourceArc<RenderPassResource>> {
        let renderpass_key = RenderPassKey {
            dsc: renderpass.clone(),
            swapchain_surface_info: swapchain_surface_info.clone(),
        };

        self.inner.render_passes.get_or_create(&renderpass_key, || {
            log::trace!("Creating renderpass\n{:#?}", renderpass_key);
            let resource = dsc::create_renderpass(
                self.inner.device_context.device(),
                &*renderpass,
                &swapchain_surface_info,
            )?;

            let resource = RenderPassResource {
                renderpass: resource,
                renderpass_key: renderpass_key.clone(),
            };

            log::trace!("Created renderpass {:?}", resource);
            Ok(resource)
        })
    }

    pub fn get_or_create_framebuffer(
        &self,
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

        self.inner.framebuffers.get_or_create(&framebuffer_key, || {
            log::trace!("Creating framebuffer\n{:#?}", framebuffer_key);

            let attachment_image_views: Vec<_> = attachments
                .iter()
                .map(|resource| resource.get_raw().image_view)
                .collect();

            let resource = dsc::create_framebuffer(
                self.inner.device_context.device(),
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
            Ok(resource)
        })
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
        &self,
        shader_modules: Vec<ResourceArc<ShaderModuleResource>>,
        shader_module_metas: Vec<dsc::ShaderModuleMeta>,
        pipeline_layout: ResourceArc<PipelineLayoutResource>,
        fixed_function_state: Arc<dsc::FixedFunctionState>,
        vertex_inputs: Arc<Vec<MaterialPassVertexInput>>,
    ) -> VkResult<ResourceArc<MaterialPassResource>> {
        let shader_module_keys = shader_modules
            .iter()
            .map(|x| x.get_raw().shader_module_key)
            .collect();
        let material_pass_key = MaterialPassKey {
            shader_module_metas,
            shader_module_keys,
            pipeline_layout: pipeline_layout.get_raw().pipeline_layout_def,
            fixed_function_state,
            vertex_inputs,
        };

        self.inner
            .material_passes
            .get_or_create(&material_pass_key, || {
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
                Ok(resource)
            })
    }

    pub fn get_or_create_graphics_pipeline(
        &self,
        material_pass: &ResourceArc<MaterialPassResource>,
        renderpass: &ResourceArc<RenderPassResource>,
        framebuffer_meta: &dsc::FramebufferMeta,
        vertex_input_state: Arc<dsc::PipelineVertexInputState>,
    ) -> VkResult<ResourceArc<GraphicsPipelineResource>> {
        let pipeline_key = GraphicsPipelineKey {
            material_pass_key: material_pass.get_raw().material_pass_key,
            renderpass_key: renderpass.get_raw().renderpass_key,
            framebuffer_meta: framebuffer_meta.clone(),
            vertex_input_state: vertex_input_state.clone(),
        };

        self.inner
            .graphics_pipelines
            .get_or_create(&pipeline_key, || {
                log::trace!("Creating pipeline\n{:#?}", pipeline_key);
                let pipelines = dsc::create_graphics_pipelines(
                    &self.inner.device_context.device(),
                    &*vertex_input_state,
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
                    &pipeline_key.framebuffer_meta,
                )?;
                log::trace!("Created pipelines {:?}", pipelines);

                let resource = GraphicsPipelineResource {
                    pipelines,
                    pipeline_layout: material_pass.get_raw().pipeline_layout,
                    renderpass: renderpass.clone(),
                };
                Ok(resource)
            })
    }

    // A key difference between this insert_image and the insert_image in a DynResourceAllocator
    // is that these can be retrieved. However, a mutable reference is required. This one is
    // more appropriate to use with descriptors loaded from assets, and DynResourceAllocator with runtime-created
    // descriptors
    pub fn insert_image(
        &self,
        image: VkImage,
    ) -> ResourceArc<ImageResource> {
        let raw_image = image.take_raw().unwrap();
        self.insert_raw_image(raw_image)
    }

    // This is useful for inserting swapchain images
    pub fn insert_raw_image(
        &self,
        raw_image: VkImageRaw,
    ) -> ResourceArc<ImageResource> {
        let image_id = self.inner.next_image_id.fetch_add(1, Ordering::Relaxed);

        let image_key = ImageKey { id: image_id };

        let resource = ImageResource {
            image: raw_image,
            image_key: Some(image_key),
        };

        self.inner
            .images
            .create(&image_key, || Ok(resource))
            .unwrap()
    }

    //TODO: Support direct removal of raw images with verification that no references remain

    // A key difference between this insert_buffer and the insert_buffer in a DynResourceAllocator
    // is that these can be retrieved. However, a mutable reference is required. This one is
    // more appropriate to use with loaded assets, and DynResourceAllocator with runtime assets
    pub fn insert_buffer(
        &self,
        buffer: ManuallyDrop<VkBuffer>,
    ) -> ResourceArc<BufferResource> {
        let buffer_id = self.inner.next_buffer_id.fetch_add(1, Ordering::Relaxed);
        let buffer_key = BufferKey { id: buffer_id };

        let raw_buffer = ManuallyDrop::into_inner(buffer).take_raw().unwrap();
        let resource = BufferResource {
            buffer: raw_buffer,
            buffer_key: Some(buffer_key),
        };

        self.inner
            .buffers
            .create(&buffer_key, || Ok(resource))
            .unwrap()
    }

    pub fn get_or_create_image_view(
        &self,
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

        self.inner.image_views.get_or_create(&image_view_key, || {
            log::trace!("Creating image view\n{:#?}", image_view_key);
            let resource = dsc::create_image_view(
                &self.inner.device_context.device(),
                image.get_raw().image.image,
                image_view_meta,
            )?;
            log::trace!("Created image view\n{:#?}", resource);

            let resource = ImageViewResource {
                image_view: resource,
                image_view_key: Some(image_view_key.clone()),
                image: image.clone(),
                image_view_meta: image_view_meta.clone(),
            };

            Ok(resource)
        })
    }

    // pub fn get_or_create_frame_buffer(
    //     &mut self,
    //     frame_buffer_meta: dsc::FrameBufferMeta,
    //     images: dsc::ImageViewMeta,
    // )
}
