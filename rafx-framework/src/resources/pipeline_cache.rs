use crate::nodes::{RenderPhase, RenderPhaseIndex, RenderRegistry, MAX_RENDER_PHASE_COUNT};
use crate::resources::resource_arc::{ResourceId, WeakResourceArc};
use crate::resources::vertex_data::{VertexDataSetLayout, VertexDataSetLayoutHash};
use crate::{GraphicsPipelineResource, MaterialPassResource, ResourceArc, ResourceLookupSet};
use fnv::{FnvHashMap, FnvHashSet, FnvHasher};
use rafx_api::{
    RafxFormat, RafxResult, RafxSampleCount, RafxVertexAttributeRate, RafxVertexLayout,
    RafxVertexLayoutAttribute, RafxVertexLayoutBuffer,
};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

//TODO: Allow caching for N frames
//TODO: Return a kind of ResourceArc for a cached pipeline. Allow dropping after N frames pass with
// nothing request/using it
//TODO: vulkan pipeline cache object

//TODO: Remove Serialize/Deserialize
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicsPipelineRenderTargetMeta {
    color_formats: Vec<RafxFormat>,
    depth_stencil_format: Option<RafxFormat>,
    sample_count: RafxSampleCount,
    hash: GraphicsPipelineRenderTargetMetaHash,
}

impl GraphicsPipelineRenderTargetMeta {
    pub fn new(
        color_formats: Vec<RafxFormat>,
        depth_stencil_format: Option<RafxFormat>,
        sample_count: RafxSampleCount,
    ) -> Self {
        let hash = GraphicsPipelineRenderTargetMetaHash::new(
            &color_formats,
            depth_stencil_format,
            sample_count,
        );
        GraphicsPipelineRenderTargetMeta {
            color_formats,
            depth_stencil_format,
            sample_count,
            hash,
        }
    }

    pub fn color_formats(&self) -> &[RafxFormat] {
        &self.color_formats
    }

    pub fn depth_stencil_format(&self) -> Option<RafxFormat> {
        self.depth_stencil_format
    }

    pub fn sample_count(&self) -> RafxSampleCount {
        self.sample_count
    }

    pub fn render_target_meta_hash(&self) -> GraphicsPipelineRenderTargetMetaHash {
        self.hash
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GraphicsPipelineRenderTargetMetaHash(u64);
impl GraphicsPipelineRenderTargetMetaHash {
    fn new(
        color_formats: &[RafxFormat],
        depth_stencil_format: Option<RafxFormat>,
        sample_count: RafxSampleCount,
    ) -> Self {
        let mut hasher = FnvHasher::default();
        color_formats.hash(&mut hasher);
        depth_stencil_format.hash(&mut hasher);
        sample_count.hash(&mut hasher);
        let hash = hasher.finish();
        GraphicsPipelineRenderTargetMetaHash(hash)
    }
}

#[derive(PartialEq, Eq, Hash)]
struct CachedGraphicsPipelineKey {
    material_pass: ResourceId,
    render_target_meta_hash: GraphicsPipelineRenderTargetMetaHash,
    vertex_data_set_layout: VertexDataSetLayoutHash,
}

#[derive(PartialEq, Eq)]
struct CachedGraphicsPipeline {
    material_pass_resource: WeakResourceArc<MaterialPassResource>,
    graphics_pipeline: ResourceArc<GraphicsPipelineResource>,
}

#[derive(Debug)]
struct RegisteredRenderTargetMeta {
    keep_until_frame: u64,
    meta: GraphicsPipelineRenderTargetMeta,
}

pub struct GraphicsPipelineCacheInner {
    resource_lookup_set: ResourceLookupSet,

    // index by render phase index
    render_target_meta_assignments:
        Vec<FnvHashMap<GraphicsPipelineRenderTargetMetaHash, RegisteredRenderTargetMeta>>,
    material_pass_assignments: Vec<FnvHashMap<ResourceId, WeakResourceArc<MaterialPassResource>>>,

    cached_pipelines: FnvHashMap<CachedGraphicsPipelineKey, CachedGraphicsPipeline>,

    current_frame_index: u64,
    frames_to_persist: u64,

    #[cfg(debug_assertions)]
    vertex_data_set_layouts: FnvHashMap<VertexDataSetLayoutHash, VertexDataSetLayout>,
    #[cfg(debug_assertions)]
    render_target_metas:
        FnvHashMap<GraphicsPipelineRenderTargetMetaHash, GraphicsPipelineRenderTargetMeta>,

    #[cfg(debug_assertions)]
    lock_call_count_previous_frame: u64,
    #[cfg(debug_assertions)]
    lock_call_count: u64,

    #[cfg(debug_assertions)]
    pipeline_create_count_previous_frame: u64,
    #[cfg(debug_assertions)]
    pipeline_create_count: u64,
}

#[derive(Debug)]
pub struct GraphicsPipelineCacheMetrics {
    pipeline_count: usize,

    #[cfg(debug_assertions)]
    lock_call_count_previous_frame: u64,
    #[cfg(debug_assertions)]
    pipeline_create_count_previous_frame: u64,
}

#[derive(Clone)]
pub struct GraphicsPipelineCache {
    render_registry: RenderRegistry,
    inner: Arc<Mutex<GraphicsPipelineCacheInner>>,
}

impl GraphicsPipelineCache {
    pub fn new(
        render_registry: &RenderRegistry,
        resource_lookup_set: ResourceLookupSet,
    ) -> Self {
        // 0 keeps to end of current frame
        // 1 keeps to end of next frame
        const DEFAULT_FRAMES_TO_PERSIST: u64 = 1;

        let mut render_target_meta_assignments =
            Vec::with_capacity(MAX_RENDER_PHASE_COUNT as usize);
        render_target_meta_assignments
            .resize_with(MAX_RENDER_PHASE_COUNT as usize, Default::default);

        let mut material_pass_assignments = Vec::with_capacity(MAX_RENDER_PHASE_COUNT as usize);
        material_pass_assignments.resize_with(MAX_RENDER_PHASE_COUNT as usize, Default::default);

        let inner = GraphicsPipelineCacheInner {
            resource_lookup_set,
            render_target_meta_assignments,
            material_pass_assignments,
            cached_pipelines: Default::default(),
            current_frame_index: 0,
            frames_to_persist: DEFAULT_FRAMES_TO_PERSIST,
            #[cfg(debug_assertions)]
            vertex_data_set_layouts: Default::default(),
            #[cfg(debug_assertions)]
            render_target_metas: Default::default(),
            #[cfg(debug_assertions)]
            lock_call_count_previous_frame: 0,
            #[cfg(debug_assertions)]
            lock_call_count: 0,
            #[cfg(debug_assertions)]
            pipeline_create_count_previous_frame: 0,
            #[cfg(debug_assertions)]
            pipeline_create_count: 0,
        };

        GraphicsPipelineCache {
            render_registry: render_registry.clone(),
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn metrics(&self) -> GraphicsPipelineCacheMetrics {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            inner.lock_call_count += 1;
        }

        GraphicsPipelineCacheMetrics {
            pipeline_count: inner.cached_pipelines.len(),
            #[cfg(debug_assertions)]
            lock_call_count_previous_frame: inner.lock_call_count_previous_frame,
            #[cfg(debug_assertions)]
            pipeline_create_count_previous_frame: inner.pipeline_create_count_previous_frame,
        }
    }

    #[cfg(debug_assertions)]
    fn verify_data_set_layout_hash_unique(
        inner: &mut GraphicsPipelineCacheInner,
        layout: &VertexDataSetLayout,
    ) {
        if let Some(previous_layout) = inner.vertex_data_set_layouts.get(&layout.hash()) {
            assert_eq!(*previous_layout, *layout);
            return;
        }

        let old = inner
            .vertex_data_set_layouts
            .insert(layout.hash(), layout.clone());
        assert!(old.is_none());
    }

    #[cfg(debug_assertions)]
    fn verify_render_target_meta_hash_unique(
        inner: &mut GraphicsPipelineCacheInner,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
    ) {
        if let Some(previous_render_target_meta) = inner
            .render_target_metas
            .get(&render_target_meta.render_target_meta_hash())
        {
            assert_eq!(*previous_render_target_meta, *render_target_meta);
            return;
        }

        let old = inner.render_target_metas.insert(
            render_target_meta.render_target_meta_hash(),
            render_target_meta.clone(),
        );
        assert!(old.is_none());
    }

    #[profiling::function]
    pub fn on_frame_complete(&self) {
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            // add one for this call
            guard.lock_call_count_previous_frame = guard.lock_call_count + 1;
            guard.lock_call_count = 0;

            guard.pipeline_create_count_previous_frame = guard.pipeline_create_count;
            guard.pipeline_create_count = 0;
        }

        // This is just removing from cache, not destroying. Resource lookup will keep them alive
        // for however long is necessary
        Self::drop_stale_pipelines(&mut *guard);
        guard.current_frame_index += 1;
    }

    pub fn get_render_phase_by_name(
        &self,
        name: &str,
    ) -> Option<RenderPhaseIndex> {
        self.render_registry.render_phase_index_from_name(name)
    }

    // Register a renderpass as being part of a particular phase. This will a pipeline is created
    // for all appropriate renderpass/material pass pairs.
    pub fn register_renderpass_to_phase_per_frame<T: RenderPhase>(
        &self,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
    ) {
        self.register_renderpass_to_phase_index_per_frame(
            render_target_meta,
            T::render_phase_index(),
        )
    }

    pub fn register_renderpass_to_phase_index_per_frame(
        &self,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        render_phase_index: RenderPhaseIndex,
    ) {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            inner.lock_call_count += 1;
        }

        Self::do_register_renderpass_to_phase_index_per_frame(
            inner,
            render_target_meta,
            render_phase_index,
        );
    }

    pub fn do_register_renderpass_to_phase_index_per_frame(
        inner: &mut GraphicsPipelineCacheInner,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        render_phase_index: RenderPhaseIndex,
    ) {
        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        if let Some(existing) = inner.render_target_meta_assignments[render_phase_index as usize]
            .get_mut(&render_target_meta.render_target_meta_hash())
        {
            existing.keep_until_frame = inner.current_frame_index + inner.frames_to_persist;
            // Nothing to do here, the previous ref is still valid
            return;
        }

        inner.render_target_meta_assignments[render_phase_index as usize].insert(
            render_target_meta.render_target_meta_hash(),
            RegisteredRenderTargetMeta {
                keep_until_frame: inner.current_frame_index + inner.frames_to_persist,
                meta: render_target_meta.clone(),
            },
        );

        //TODO: Do we need to mark this as a dirty renderpass that may need to build additional
        // pipelines?
    }

    pub fn register_material_to_phase_index(
        &self,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_phase_index: RenderPhaseIndex,
    ) {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            inner.lock_call_count += 1;
        }

        Self::do_register_material_to_phase_index(inner, material_pass, render_phase_index);
    }

    pub fn do_register_material_to_phase_index(
        inner: &mut GraphicsPipelineCacheInner,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_phase_index: RenderPhaseIndex,
    ) {
        // May be caused by not registering a render phase before using it
        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        if let Some(existing) = inner.material_pass_assignments[render_phase_index as usize]
            .get(&material_pass.get_hash())
        {
            if existing.upgrade().is_some() {
                // Nothing to do here, the previous ref is still valid
                return;
            }
        }

        inner.material_pass_assignments[render_phase_index as usize]
            .insert(material_pass.get_hash(), material_pass.downgrade());
        //TODO: Do we need to mark this as a dirty material that may need to build additional
        // pipelines?
    }

    pub fn try_get_graphics_pipeline(
        &self,
        render_phase_index: RenderPhaseIndex,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        vertex_data_set_layout: &VertexDataSetLayout,
    ) -> Option<ResourceArc<GraphicsPipelineResource>> {
        // RafxResult is always Ok if returning cached pipelines
        self.graphics_pipeline(
            render_phase_index,
            material_pass,
            render_target_meta,
            vertex_data_set_layout,
            false,
        )
        .map(|x| x.unwrap())
    }

    pub fn get_or_create_graphics_pipeline(
        &self,
        render_phase_index: RenderPhaseIndex,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        vertex_data_set_layout: &VertexDataSetLayout,
    ) -> RafxResult<ResourceArc<GraphicsPipelineResource>> {
        // graphics_pipeline never returns none if create_if_missing is true
        self.graphics_pipeline(
            render_phase_index,
            material_pass,
            render_target_meta,
            vertex_data_set_layout,
            true,
        )
        .ok_or("Failed to create graphics pipeline")?
    }

    pub fn graphics_pipeline(
        &self,
        render_phase_index: RenderPhaseIndex,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_target_meta: &GraphicsPipelineRenderTargetMeta,
        vertex_data_set_layout: &VertexDataSetLayout,
        create_if_missing: bool,
    ) -> Option<RafxResult<ResourceArc<GraphicsPipelineResource>>> {
        let key = CachedGraphicsPipelineKey {
            material_pass: material_pass.get_hash(),
            render_target_meta_hash: render_target_meta.render_target_meta_hash(),
            vertex_data_set_layout: vertex_data_set_layout.hash(),
        };

        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            Self::verify_data_set_layout_hash_unique(inner, vertex_data_set_layout);
            Self::verify_render_target_meta_hash_unique(inner, render_target_meta);
            inner.lock_call_count += 1;
        }

        Self::do_register_renderpass_to_phase_index_per_frame(
            inner,
            render_target_meta,
            render_phase_index,
        );
        Self::do_register_material_to_phase_index(inner, material_pass, render_phase_index);

        inner
            .cached_pipelines
            .get(&key)
            .map(|x| {
                debug_assert!(x.material_pass_resource.upgrade().is_some());
                Ok(x.graphics_pipeline.clone())
            })
            .or_else(|| {
                if create_if_missing {
                    log::debug!("Creating graphics pipeline");
                    profiling::scope!("Create Pipeline");
                    //let mut binding_descriptions = Vec::default();
                    let mut vertex_layout_buffers =
                        Vec::with_capacity(vertex_data_set_layout.bindings().len());
                    for binding in vertex_data_set_layout.bindings() {
                        vertex_layout_buffers.push(RafxVertexLayoutBuffer {
                            rate: RafxVertexAttributeRate::Vertex,
                            stride: binding.vertex_stride() as u32,
                        })
                    }

                    //let mut attribute_descriptions = Vec::default();
                    let mut vertex_layout_attributes =
                        Vec::with_capacity(material_pass.get_raw().vertex_inputs.len());

                    for vertex_input in &*material_pass.get_raw().vertex_inputs {
                        let member = vertex_data_set_layout
                            .member(&vertex_input.semantic)
                            .ok_or_else(|| {
                                let error_message = format!(
                                    "Vertex data does not support this material. Missing data {}",
                                    vertex_input.semantic
                                );
                                log::error!("{}", error_message);
                                log::info!(
                                    "  required inputs:\n{:#?}",
                                    material_pass.get_raw().vertex_inputs
                                );
                                log::info!(
                                    "  available inputs:\n{:#?}",
                                    vertex_data_set_layout.members()
                                );
                                error_message
                            })
                            .ok()?;

                        vertex_layout_attributes.push(RafxVertexLayoutAttribute {
                            location: vertex_input.location,
                            byte_offset: member.byte_offset as u32,
                            buffer_index: member.binding as u32,
                            format: member.format,
                            gl_attribute_name: Some(vertex_input.gl_attribute_name.clone()),
                        });
                    }

                    let vertex_layout = RafxVertexLayout {
                        attributes: vertex_layout_attributes,
                        buffers: vertex_layout_buffers,
                    };

                    log::trace!("Creating graphics pipeline. Setting up vertex formats:");
                    log::trace!(
                        "  required inputs:\n{:#?}",
                        material_pass.get_raw().vertex_inputs
                    );
                    log::trace!(
                        "  available inputs:\n{:#?}",
                        vertex_data_set_layout.members()
                    );

                    #[cfg(debug_assertions)]
                    {
                        inner.pipeline_create_count += 1;
                    }

                    log::trace!("Create vertex layout {:#?}", vertex_layout);
                    let pipeline = inner.resource_lookup_set.get_or_create_graphics_pipeline(
                        &material_pass,
                        render_target_meta,
                        vertex_data_set_layout.primitive_topology(),
                        &vertex_layout,
                    );

                    if let Ok(pipeline) = pipeline {
                        inner.cached_pipelines.insert(
                            key,
                            CachedGraphicsPipeline {
                                graphics_pipeline: pipeline.clone(),
                                //render_target_meta: render_target_meta.clone(),
                                material_pass_resource: material_pass.downgrade(),
                            },
                        );

                        Some(Ok(pipeline))
                    } else {
                        Some(pipeline)
                    }
                } else {
                    None
                }
            })
    }

    pub fn precache_pipelines_for_all_phases(&self) -> RafxResult<()> {
        let mut guard = self.inner.lock().unwrap();
        let _inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            _inner.lock_call_count += 1;
        }

        //TODO: Avoid iterating everything all the time
        //TODO: This will have to be reworked to include vertex layout as part of the key. Current
        // plan is to register vertex types in code with the registry and have materials reference
        // them by name
        /*
        for render_phase_index in 0..MAX_RENDER_PHASE_COUNT {
            for (render_target_meta, renderpass) in
                &inner.render_target_meta_assignments[render_phase_index as usize]
            {
                for (material_pass_hash, material_pass) in
                    &inner.material_pass_assignments[render_phase_index as usize]
                {
                    let key = CachedGraphicsPipelineKey {
                        renderpass: *renderpass_hash,
                        material_pass: *material_pass_hash,
                    };

                    if !inner.cached_pipelines.contains_key(&key) {
                        if let Some(renderpass) = renderpass.renderpass.upgrade() {
                            if let Some(material_pass) = material_pass.upgrade() {
                                #[cfg(debug_assertions)]
                                {
                                    guard.pipeline_create_count += 1;
                                }

                                let pipeline = inner
                                    .resource_lookup_set
                                    .get_or_create_graphics_pipeline(&material_pass, &renderpass)?;
                                inner.cached_pipelines.insert(
                                    key,
                                    CachedGraphicsPipeline {
                                        graphics_pipeline: pipeline,
                                        renderpass_resource: renderpass.downgrade(),
                                        material_pass_resource: material_pass.downgrade(),
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        */

        Ok(())
    }

    fn drop_stale_pipelines(inner: &mut GraphicsPipelineCacheInner) {
        let current_frame_index = inner.current_frame_index;

        for phase in &mut inner.render_target_meta_assignments {
            phase.retain(|_k, v| v.keep_until_frame > current_frame_index);
        }

        for phase in &mut inner.material_pass_assignments {
            phase.retain(|_k, v| v.upgrade().is_some());
        }

        //TODO: Could do something smarter than this to track when the last one is dropped
        let mut all_render_target_meta = FnvHashSet::default();
        for phase in &inner.render_target_meta_assignments {
            for key in phase.keys() {
                all_render_target_meta.insert(key);
            }
        }

        inner.cached_pipelines.retain(|k, v| {
            let render_target_meta_still_exists = all_render_target_meta.contains(&k.render_target_meta_hash);
            let material_pass_still_exists = v.material_pass_resource.upgrade().is_some();

            if !render_target_meta_still_exists || !material_pass_still_exists {
                log::debug!(
                    "Dropping pipeline from cache, render_target_meta_still_exists: {}, material_pass_still_exists: {}",
                    render_target_meta_still_exists,
                    material_pass_still_exists
                );
            }

            render_target_meta_still_exists && material_pass_still_exists
        })
    }

    pub fn clear_all_pipelines(&self) {
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count += 1;
        }

        guard.cached_pipelines.clear();
    }
}
