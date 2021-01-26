use crate::resources::pipeline_cache::GraphicsPipelineRenderTargetMeta;
use crate::resources::resource_arc::{ResourceId, ResourceWithHash, WeakResourceArc};
use crate::resources::DescriptorSetLayout;
use crate::resources::ResourceArc;
use crate::ResourceDropSink;
use crossbeam_channel::{Receiver, Sender};
use fnv::{FnvHashMap, FnvHasher};
use rafx_api::extra::image::RafxImage;
use rafx_api::*;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

// Hash of a GPU resource
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ResourceHash(u64);

impl ResourceHash {
    pub fn from_key<KeyT: Hash>(key: &KeyT) -> ResourceHash {
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
    ResourceT: Clone,
{
    resources: FnvHashMap<ResourceHash, WeakResourceArc<ResourceT>>,
    //TODO: Add support for "cancelling" dropping stuff. This would likely be a ring of hashmaps.
    // that gets cycled.
    drop_sink: ResourceDropSink<ResourceT>,
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
    ResourceT: Clone,
{
    inner: Mutex<ResourceLookupInner<KeyT, ResourceT>>,
}

impl<KeyT, ResourceT> ResourceLookup<KeyT, ResourceT>
where
    KeyT: Eq + Hash + Clone,
    ResourceT: Clone + std::fmt::Debug,
{
    pub fn new(max_frames_in_flight: u32) -> Self {
        let (drop_tx, drop_rx) = crossbeam_channel::unbounded();

        let inner = ResourceLookupInner {
            resources: Default::default(),
            drop_sink: ResourceDropSink::new(max_frames_in_flight),
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
    ) -> RafxResult<ResourceArc<ResourceT>>
    where
        F: FnOnce() -> RafxResult<ResourceT>,
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

    #[allow(dead_code)]
    pub fn get(
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

    pub fn create<F>(
        &self,
        key: &KeyT,
        create_resource_fn: F,
    ) -> RafxResult<ResourceArc<ResourceT>>
    where
        F: FnOnce() -> RafxResult<ResourceT>,
    {
        let hash = ResourceHash::from_key(key);
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count += 1;
        }

        Self::do_create(&mut *guard, hash, key, create_resource_fn)
    }

    pub fn get_or_create<F>(
        &self,
        key: &KeyT,
        create_resource_fn: F,
    ) -> RafxResult<ResourceArc<ResourceT>>
    where
        F: FnOnce() -> RafxResult<ResourceT>,
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

    fn on_frame_complete(&self) -> RafxResult<()> {
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count_previous_frame = guard.lock_call_count + 1;
            guard.lock_call_count = 0;
        }

        guard.create_count_previous_frame = guard.create_count;
        guard.create_count = 0;

        Self::handle_dropped_resources(&mut guard);
        guard.drop_sink.on_frame_complete()?;
        Ok(())
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

    fn destroy(&self) -> RafxResult<()> {
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

        guard.drop_sink.destroy()?;
        Ok(())
    }
}

//
// Keys for each resource type. (Some keys are simple and use types from crate::pipeline_description
// and some are a combination of the definitions and runtime state. (For example, combining a
// renderpass with the swapchain surface it would be applied to)
//

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FixedFunctionState {
    pub blend_state: RafxBlendState,
    pub depth_state: RafxDepthState,
    pub rasterizer_state: RafxRasterizerState,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShaderModuleMeta {
    pub stage: RafxShaderStageFlags,
    pub entry_name: String,
    // Reference to shader is excluded
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct ShaderModuleResourceDef {
    // Precalculate a hash so we can avoid hashing this blob of bytes at runtime
    pub shader_module_hash: ShaderModuleHash,
    pub shader_package: RafxShaderPackage,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShaderModuleHash(u64);
impl ShaderModuleHash {
    pub fn new(shader_package: &RafxShaderPackage) -> Self {
        let mut hasher = FnvHasher::default();
        shader_package.hash(&mut hasher);
        let hash = hasher.finish();
        ShaderModuleHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ShaderHash(u64);
impl ShaderHash {
    pub fn new(
        stage_defs: &[RafxShaderStageDef],
        shader_module_hashes: &[ShaderModuleHash],
    ) -> Self {
        let mut hasher = FnvHasher::default();
        RafxShaderStageDef::hash_definition(&mut hasher, stage_defs, shader_module_hashes);
        let hash = hasher.finish();
        ShaderHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SamplerHash(u64);
impl SamplerHash {
    pub fn new(sampler_def: &RafxSamplerDef) -> Self {
        let mut hasher = FnvHasher::default();
        sampler_def.hash(&mut hasher);
        let hash = hasher.finish();
        SamplerHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct RootSignatureHash(u64);
impl RootSignatureHash {
    pub fn new(
        shader_hashes: &[ShaderHash],
        immutable_sampler_keys: &[RafxImmutableSamplerKey],
        immutable_sampler_hashes: &[Vec<SamplerHash>],
    ) -> Self {
        let mut hasher = FnvHasher::default();
        RafxRootSignatureDef::hash_definition(
            &mut hasher,
            shader_hashes,
            immutable_sampler_keys,
            immutable_sampler_hashes,
        );
        let hash = hasher.finish();
        RootSignatureHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DescriptorSetLayoutHash(u64);
impl DescriptorSetLayoutHash {
    pub fn new(
        root_signature_hash: RootSignatureHash,
        set_index: u32,
        bindings: &DescriptorSetLayout,
    ) -> Self {
        let mut hasher = FnvHasher::default();
        root_signature_hash.hash(&mut hasher);
        set_index.hash(&mut hasher);
        bindings.hash(&mut hasher);
        let hash = hasher.finish();
        DescriptorSetLayoutHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct MaterialPassHash(u64);
impl MaterialPassHash {
    pub fn new(
        shader_hash: ShaderHash,
        root_signature_hash: RootSignatureHash,
        descriptor_set_layout_hashes: &[DescriptorSetLayoutHash],
        fixed_function_state: &FixedFunctionState,
        vertex_inputs: &[MaterialPassVertexInput],
    ) -> Self {
        let mut hasher = FnvHasher::default();
        shader_hash.hash(&mut hasher);
        root_signature_hash.hash(&mut hasher);
        descriptor_set_layout_hashes.hash(&mut hasher);
        fixed_function_state.hash(&mut hasher);
        for vertex_input in vertex_inputs {
            vertex_input.hash(&mut hasher);
        }
        let hash = hasher.finish();
        MaterialPassHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct GraphicsPipelineHash(u64);
impl GraphicsPipelineHash {
    pub fn new(
        material_pass_key: MaterialPassHash,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        primitive_topology: RafxPrimitiveTopology,
        vertex_layout: &RafxVertexLayout,
    ) -> Self {
        let mut hasher = FnvHasher::default();
        material_pass_key.hash(&mut hasher);
        render_target_meta
            .render_target_meta_hash()
            .hash(&mut hasher);
        primitive_topology.hash(&mut hasher);
        vertex_layout.hash(&mut hasher);
        let hash = hasher.finish();
        GraphicsPipelineHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ComputePipelineHash(u64);
impl ComputePipelineHash {
    pub fn new(
        shader_hash: ShaderHash,
        root_signature_hash: RootSignatureHash,
        descriptor_set_layout_hashes: &[DescriptorSetLayoutHash],
    ) -> Self {
        let mut hasher = FnvHasher::default();
        shader_hash.hash(&mut hasher);
        root_signature_hash.hash(&mut hasher);
        descriptor_set_layout_hashes.hash(&mut hasher);
        let hash = hasher.finish();
        ComputePipelineHash(hash)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderModuleKey {
    hash: ShaderModuleHash,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShaderKey {
    hash: ShaderHash,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RootSignatureKey {
    // hash is based on shader code hash, stage, and entry point
    hash: RootSignatureHash,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayoutKey {
    hash: DescriptorSetLayoutHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialPassVertexInput {
    pub semantic: String,
    pub location: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialPassKey {
    hash: MaterialPassHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineKey {
    hash: GraphicsPipelineHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComputePipelineKey {
    hash: ComputePipelineHash,
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
    hash: SamplerHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageViewKey {
    image_key: ImageKey,
    texture_bind_type: Option<RafxTextureBindType>,
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
    pub shader_metrics: ResourceLookupMetric,
    pub root_signature_metrics: ResourceLookupMetric,
    pub descriptor_set_layout_metrics: ResourceLookupMetric,
    pub material_pass_metrics: ResourceLookupMetric,
    pub graphics_pipeline_metrics: ResourceLookupMetric,
    pub compute_pipeline_metrics: ResourceLookupMetric,
    pub image_metrics: ResourceLookupMetric,
    pub image_view_metrics: ResourceLookupMetric,
    pub sampler_metrics: ResourceLookupMetric,
    pub buffer_metrics: ResourceLookupMetric,
}

#[derive(Debug, Clone)]
pub struct ShaderModuleResource {
    pub shader_module_key: ShaderModuleKey,
    pub shader_module_resource_def: Arc<ShaderModuleResourceDef>,
    pub shader_module: RafxShaderModule,
}

#[derive(Debug, Clone)]
pub struct ShaderResource {
    pub key: ShaderKey,
    pub shader_modules: Vec<ResourceArc<ShaderModuleResource>>,
    pub shader: RafxShader,
}

#[derive(Debug, Clone)]
pub struct RootSignatureResource {
    pub key: RootSignatureKey,
    pub shaders: Vec<ResourceArc<ShaderResource>>,
    pub immutable_samplers: Vec<ResourceArc<SamplerResource>>,
    pub root_signature: RafxRootSignature,
}

#[derive(Debug, Clone)]
pub struct DescriptorSetLayoutResource {
    // Just keep it in scope
    pub root_signature_arc: ResourceArc<RootSignatureResource>,
    pub root_signature: RafxRootSignature,
    pub set_index: u32,

    pub descriptor_set_layout_def: DescriptorSetLayout,
    pub key: DescriptorSetLayoutKey,
}

#[derive(Debug, Clone)]
pub struct MaterialPassResource {
    pub material_pass_key: MaterialPassKey,
    pub shader: ResourceArc<ShaderResource>,
    pub root_signature: ResourceArc<RootSignatureResource>,
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,

    pub fixed_function_state: Arc<FixedFunctionState>,
    pub vertex_inputs: Arc<Vec<MaterialPassVertexInput>>,
}

#[derive(Debug, Clone)]
pub struct GraphicsPipelineResource {
    pub render_target_meta: GraphicsPipelineRenderTargetMeta,
    pub pipeline: Arc<RafxPipeline>,
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
}

#[derive(Debug, Clone)]
pub struct ComputePipelineResource {
    pub root_signature: ResourceArc<RootSignatureResource>,
    pub pipeline: Arc<RafxPipeline>,
    pub descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
}

#[derive(Debug, Clone)]
pub struct ImageResource {
    pub image: Arc<RafxImage>,
    // Dynamic resources have no key
    pub image_key: Option<ImageKey>,
}

#[derive(Debug, Clone)]
pub struct ImageViewResource {
    pub image: ResourceArc<ImageResource>,
    // Dynamic resources have no key
    pub image_view_key: Option<ImageViewKey>,
    pub texture_bind_type: Option<RafxTextureBindType>,
}

#[derive(Debug, Clone)]
pub struct SamplerResource {
    pub sampler: RafxSampler,
    pub sampler_key: SamplerKey,
}

#[derive(Debug, Clone)]
pub struct BufferResource {
    pub buffer: Arc<RafxBuffer>,
    // Dynamic resources have no key
    pub buffer_key: Option<BufferKey>,
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
    device_context: RafxDeviceContext,

    shader_modules: ResourceLookup<ShaderModuleKey, ShaderModuleResource>,
    shaders: ResourceLookup<ShaderKey, ShaderResource>,
    root_signatures: ResourceLookup<RootSignatureKey, RootSignatureResource>,
    descriptor_set_layouts: ResourceLookup<DescriptorSetLayoutKey, DescriptorSetLayoutResource>,
    material_passes: ResourceLookup<MaterialPassKey, MaterialPassResource>,
    graphics_pipelines: ResourceLookup<GraphicsPipelineKey, GraphicsPipelineResource>,
    compute_pipelines: ResourceLookup<ComputePipelineKey, ComputePipelineResource>,
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
        device_context: &RafxDeviceContext,
        max_frames_in_flight: u32,
    ) -> Self {
        let set = ResourceLookupSetInner {
            device_context: device_context.clone(),
            shader_modules: ResourceLookup::new(max_frames_in_flight),
            shaders: ResourceLookup::new(max_frames_in_flight),
            root_signatures: ResourceLookup::new(max_frames_in_flight),
            descriptor_set_layouts: ResourceLookup::new(max_frames_in_flight),
            material_passes: ResourceLookup::new(max_frames_in_flight),
            graphics_pipelines: ResourceLookup::new(max_frames_in_flight),
            compute_pipelines: ResourceLookup::new(max_frames_in_flight),
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
    pub fn on_frame_complete(&self) -> RafxResult<()> {
        self.inner.images.on_frame_complete()?;
        self.inner.image_views.on_frame_complete()?;
        self.inner.buffers.on_frame_complete()?;
        self.inner.shader_modules.on_frame_complete()?;
        self.inner.shaders.on_frame_complete()?;
        self.inner.samplers.on_frame_complete()?;
        self.inner.root_signatures.on_frame_complete()?;
        self.inner.descriptor_set_layouts.on_frame_complete()?;
        self.inner.material_passes.on_frame_complete()?;
        self.inner.graphics_pipelines.on_frame_complete()?;
        self.inner.compute_pipelines.on_frame_complete()?;
        Ok(())
    }

    // This assumes that no GPU work remains that relies on these resources. Use
    // RafxQueue::wait_for_queue_idle
    pub fn destroy(&self) -> RafxResult<()> {
        //WARNING: These need to be in order of dependencies to avoid frame-delays on destroying
        // resources.
        self.inner.compute_pipelines.destroy()?;
        self.inner.graphics_pipelines.destroy()?;
        self.inner.material_passes.destroy()?;
        self.inner.descriptor_set_layouts.destroy()?;
        self.inner.root_signatures.destroy()?;
        self.inner.samplers.destroy()?;
        self.inner.shaders.destroy()?;
        self.inner.shader_modules.destroy()?;
        self.inner.buffers.destroy()?;
        self.inner.image_views.destroy()?;
        self.inner.images.destroy()?;
        Ok(())
    }

    pub fn metrics(&self) -> ResourceMetrics {
        ResourceMetrics {
            shader_module_metrics: self.inner.shader_modules.metrics(),
            shader_metrics: self.inner.shaders.metrics(),
            root_signature_metrics: self.inner.root_signatures.metrics(),
            descriptor_set_layout_metrics: self.inner.descriptor_set_layouts.metrics(),
            material_pass_metrics: self.inner.material_passes.metrics(),
            graphics_pipeline_metrics: self.inner.graphics_pipelines.metrics(),
            compute_pipeline_metrics: self.inner.compute_pipelines.metrics(),
            image_metrics: self.inner.images.metrics(),
            image_view_metrics: self.inner.image_views.metrics(),
            sampler_metrics: self.inner.samplers.metrics(),
            buffer_metrics: self.inner.buffers.metrics(),
        }
    }

    pub fn get_or_create_shader_module(
        &self,
        shader_module_resource_def: &Arc<ShaderModuleResourceDef>,
    ) -> RafxResult<ResourceArc<ShaderModuleResource>> {
        let shader_module_key = ShaderModuleKey {
            hash: shader_module_resource_def.shader_module_hash,
        };

        self.inner
            .shader_modules
            .get_or_create(&shader_module_key, || {
                log::trace!(
                    "Creating shader module\n[hash: {:?}]",
                    shader_module_key.hash,
                );

                let shader_module = self
                    .inner
                    .device_context
                    .create_shader_module(shader_module_resource_def.shader_package.module_def())?;

                let resource = ShaderModuleResource {
                    shader_module,
                    shader_module_resource_def: shader_module_resource_def.clone(),
                    shader_module_key: shader_module_key.clone(),
                };
                log::trace!("Created shader module {:?}", resource);
                Ok(resource)
            })
    }

    pub fn get_or_create_sampler(
        &self,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<ResourceArc<SamplerResource>> {
        let hash = SamplerHash::new(sampler_def);
        let sampler_key = SamplerKey { hash };

        self.inner.samplers.get_or_create(&sampler_key, || {
            log::trace!("Creating sampler\n{:#?}", sampler_def);

            let sampler = self.inner.device_context.create_sampler(sampler_def)?;

            let resource = SamplerResource {
                sampler,
                sampler_key: sampler_key.clone(),
            };

            log::trace!("Created sampler {:?}", resource);
            Ok(resource)
        })
    }

    pub fn get_or_create_shader(
        &self,
        shader_stage_defs: &[RafxShaderStageDef],
        shader_modules: &[ResourceArc<ShaderModuleResource>],
    ) -> RafxResult<ResourceArc<ShaderResource>> {
        let shader_module_hashes: Vec<_> = shader_modules
            .iter()
            .map(|x| x.get_raw().shader_module_key.hash)
            .collect();
        let hash = ShaderHash::new(shader_stage_defs, &shader_module_hashes);
        let key = ShaderKey { hash };

        self.inner.shaders.get_or_create(&key, || {
            log::trace!("Creating sampler\n{:#?}", shader_stage_defs);

            let shader = self
                .inner
                .device_context
                .create_shader(shader_stage_defs.iter().cloned().collect())?;

            let resource = ShaderResource {
                key,
                shader,
                shader_modules: shader_modules.iter().cloned().collect(),
            };

            log::trace!("Created sampler {:?}", resource);
            Ok(resource)
        })
    }

    pub fn get_or_create_root_signature(
        &self,
        shader_resources: &[ResourceArc<ShaderResource>],
        immutable_sampler_keys: &[RafxImmutableSamplerKey],
        immutable_sampler_resources: &[Vec<ResourceArc<SamplerResource>>],
    ) -> RafxResult<ResourceArc<RootSignatureResource>> {
        let shader_hashes: Vec<_> = shader_resources
            .iter()
            .map(|x| x.get_raw().key.hash)
            .collect();

        let mut sampler_hashes = Vec::with_capacity(immutable_sampler_resources.len());
        for sampler_list in immutable_sampler_resources {
            let hashes: Vec<_> = sampler_list
                .iter()
                .map(|x| x.get_raw().sampler_key.hash)
                .collect();
            sampler_hashes.push(hashes);
        }

        let hash = RootSignatureHash::new(&shader_hashes, immutable_sampler_keys, &sampler_hashes);
        let key = RootSignatureKey { hash };

        self.inner.root_signatures.get_or_create(&key, || {
            let mut samplers = Vec::with_capacity(immutable_sampler_resources.len());
            for sampler_list in immutable_sampler_resources {
                let cloned_sampler_list: Vec<_> = sampler_list
                    .iter()
                    .map(|x| x.get_raw().sampler.clone())
                    .collect();
                samplers.push(cloned_sampler_list);
            }

            let mut immutable_samplers = Vec::with_capacity(samplers.len());
            for i in 0..samplers.len() {
                immutable_samplers.push(RafxImmutableSamplers {
                    key: immutable_sampler_keys[i].clone(),
                    samplers: &samplers[i],
                });
            }

            log::trace!("Creating root signature\n{:#?}", key);
            let shaders: Vec<_> = shader_resources
                .iter()
                .map(|x| x.get_raw().shader.clone())
                .collect();
            let root_signature =
                self.inner
                    .device_context
                    .create_root_signature(&RafxRootSignatureDef {
                        shaders: &shaders,
                        immutable_samplers: &immutable_samplers,
                    })?;

            let shaders = shader_resources.iter().cloned().collect();

            let mut immutable_samplers = vec![];
            for resource_list in immutable_sampler_resources {
                for resource in resource_list {
                    immutable_samplers.push(resource.clone());
                }
            }

            let resource = RootSignatureResource {
                key,
                root_signature,
                shaders,
                immutable_samplers,
            };

            log::trace!("Created root signature");
            Ok(resource)
        })
    }

    pub fn get_or_create_descriptor_set_layout(
        &self,
        root_signature: &ResourceArc<RootSignatureResource>,
        set_index: u32,
        descriptor_set_layout_def: &DescriptorSetLayout,
    ) -> RafxResult<ResourceArc<DescriptorSetLayoutResource>> {
        let hash = DescriptorSetLayoutHash::new(
            root_signature.get_raw().key.hash,
            set_index,
            descriptor_set_layout_def,
        );
        let key = DescriptorSetLayoutKey { hash };

        self.inner.descriptor_set_layouts.get_or_create(&key, || {
            log::trace!(
                "Creating descriptor set layout set_index={}, root_signature:\n{:#?}",
                set_index,
                root_signature
            );

            // Create the resource object, which contains the descriptor set layout we created plus
            // ResourceArcs to the samplers, which must remain alive for the lifetime of the descriptor set
            let resource = DescriptorSetLayoutResource {
                root_signature_arc: root_signature.clone(),
                root_signature: root_signature.get_raw().root_signature.clone(),
                set_index,
                descriptor_set_layout_def: descriptor_set_layout_def.clone(),
                key: key.clone(),
            };

            log::trace!("Created descriptor set layout {:?}", resource);
            Ok(resource)
        })
    }

    pub fn get_or_create_material_pass(
        &self,
        shader: ResourceArc<ShaderResource>,
        root_signature: ResourceArc<RootSignatureResource>,
        descriptor_sets: Vec<ResourceArc<DescriptorSetLayoutResource>>,
        fixed_function_state: Arc<FixedFunctionState>,
        vertex_inputs: Arc<Vec<MaterialPassVertexInput>>,
    ) -> RafxResult<ResourceArc<MaterialPassResource>> {
        let descriptor_set_hashes: Vec<_> = descriptor_sets
            .iter()
            .map(|x| x.get_raw().key.hash)
            .collect();
        let hash = MaterialPassHash::new(
            shader.get_raw().key.hash,
            root_signature.get_raw().key.hash,
            &descriptor_set_hashes,
            &*fixed_function_state,
            &*vertex_inputs,
        );
        let material_pass_key = MaterialPassKey { hash };

        self.inner
            .material_passes
            .get_or_create(&material_pass_key, || {
                log::trace!("Creating material pass\n{:#?}", material_pass_key);
                let resource = MaterialPassResource {
                    material_pass_key: material_pass_key.clone(),
                    root_signature,
                    descriptor_set_layouts: descriptor_sets,
                    shader,
                    fixed_function_state,
                    vertex_inputs,
                };
                Ok(resource)
            })
    }

    pub fn get_or_create_graphics_pipeline(
        &self,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        primitive_topology: RafxPrimitiveTopology,
        vertex_layout: &RafxVertexLayout,
    ) -> RafxResult<ResourceArc<GraphicsPipelineResource>> {
        let hash = GraphicsPipelineHash::new(
            material_pass.get_raw().material_pass_key.hash,
            render_target_meta,
            primitive_topology,
            vertex_layout,
        );

        let pipeline_key = GraphicsPipelineKey { hash };

        self.inner
            .graphics_pipelines
            .get_or_create(&pipeline_key, || {
                log::trace!("Creating graphics pipeline\n{:#?}", pipeline_key);

                let fixed_function_state = &material_pass.get_raw().fixed_function_state;
                let pipeline = self.inner.device_context.create_graphics_pipeline(
                    &RafxGraphicsPipelineDef {
                        root_signature: &material_pass
                            .get_raw()
                            .root_signature
                            .get_raw()
                            .root_signature,

                        shader: &material_pass.get_raw().shader.get_raw().shader,

                        blend_state: &fixed_function_state.blend_state,
                        depth_state: &fixed_function_state.depth_state,
                        rasterizer_state: &fixed_function_state.rasterizer_state,

                        primitive_topology,
                        vertex_layout: &vertex_layout,

                        color_formats: &render_target_meta.color_formats(),
                        depth_stencil_format: render_target_meta.depth_stencil_format(),
                        sample_count: render_target_meta.sample_count(),
                    },
                )?;

                let resource = GraphicsPipelineResource {
                    render_target_meta: render_target_meta.clone(),
                    pipeline: Arc::new(pipeline),
                    descriptor_set_layouts: material_pass.get_raw().descriptor_set_layouts.clone(),
                };
                Ok(resource)
            })
    }

    pub fn get_or_create_compute_pipeline(
        &self,
        shader: &ResourceArc<ShaderResource>,
        root_signature: &ResourceArc<RootSignatureResource>,
        descriptor_set_layouts: Vec<ResourceArc<DescriptorSetLayoutResource>>,
    ) -> RafxResult<ResourceArc<ComputePipelineResource>> {
        let descriptor_set_hashes: Vec<_> = descriptor_set_layouts
            .iter()
            .map(|x| x.get_raw().key.hash)
            .collect();
        let hash = ComputePipelineHash::new(
            shader.get_raw().key.hash,
            root_signature.get_raw().key.hash,
            &descriptor_set_hashes,
        );
        let pipeline_key = ComputePipelineKey { hash };

        self.inner
            .compute_pipelines
            .get_or_create(&pipeline_key, || {
                log::trace!("Creating compute pipeline\n{:#?}", pipeline_key);
                let rafx_pipeline =
                    self.inner
                        .device_context
                        .create_compute_pipeline(&RafxComputePipelineDef {
                            root_signature: &root_signature.get_raw().root_signature,
                            shader: &shader.get_raw().shader,
                        })?;
                log::trace!("Created compute pipeline {:?}", rafx_pipeline);

                let resource = ComputePipelineResource {
                    root_signature: root_signature.clone(),
                    pipeline: Arc::new(rafx_pipeline),
                    descriptor_set_layouts,
                };
                Ok(resource)
            })
    }

    //
    // A key difference between these insert_image and the insert_image in a DynResourceAllocator
    // is that these can be retrieved. However, a mutable reference is required. This one is
    // more appropriate to use with descriptors loaded from assets, and DynResourceAllocator with runtime-created
    // descriptors
    //
    pub fn insert_texture(
        &self,
        texture: RafxTexture,
    ) -> ResourceArc<ImageResource> {
        let image = RafxImage::Texture(texture);
        self.insert_image(image)
    }

    pub fn insert_render_target(
        &self,
        render_target: RafxRenderTarget,
    ) -> ResourceArc<ImageResource> {
        let image = RafxImage::RenderTarget(render_target);
        self.insert_image(image)
    }

    pub fn insert_image(
        &self,
        image: RafxImage,
    ) -> ResourceArc<ImageResource> {
        let image_id = self.inner.next_image_id.fetch_add(1, Ordering::Relaxed);

        let image_key = ImageKey { id: image_id };

        let resource = ImageResource {
            image: Arc::new(image),
            image_key: Some(image_key),
        };

        self.inner
            .images
            .create(&image_key, || Ok(resource))
            .unwrap()
    }

    //TODO: Support direct removal of raw images with verification that no references remain

    // A key difference between this insert_buffer and the insert_buffer in a DynResourceAllocator
    // is that these can be retrieved. This one is more appropriate to use with loaded assets, and
    // DynResourceAllocator with runtime assets
    pub fn insert_buffer(
        &self,
        buffer: RafxBuffer,
    ) -> ResourceArc<BufferResource> {
        let buffer_id = self.inner.next_buffer_id.fetch_add(1, Ordering::Relaxed);
        let buffer_key = BufferKey { id: buffer_id };

        let resource = BufferResource {
            buffer: Arc::new(buffer),
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
        texture_bind_type: Option<RafxTextureBindType>,
    ) -> RafxResult<ResourceArc<ImageViewResource>> {
        if image.get_raw().image_key.is_none() {
            log::error!("Tried to create an image view resource with a dynamic image");
            return Err("Tried to create an image view resource with a dynamic image")?;
        }

        let image_view_key = ImageViewKey {
            image_key: image.get_raw().image_key.unwrap(),
            texture_bind_type,
        };

        self.inner.image_views.get_or_create(&image_view_key, || {
            log::trace!("Creating image view\n{:#?}", image_view_key);
            let resource = ImageViewResource {
                image: image.clone(),
                texture_bind_type,
                image_view_key: Some(image_view_key.clone()),
            };
            log::trace!("Created image view\n{:#?}", resource);

            Ok(resource)
        })
    }
}
