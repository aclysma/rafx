use crate::{
    ResourceLookupSet, RenderPassResource, ResourceArc, GraphicsPipelineResource,
    MaterialPassResource,
};
use renderer_nodes::{RenderPhase, RenderPhaseIndex, MAX_RENDER_PHASE_COUNT, RenderRegistry};
use crate::resources::resource_arc::{WeakResourceArc, ResourceId};
use fnv::FnvHashMap;
use std::hash::Hash;
use ash::prelude::VkResult;
use std::sync::{Arc, Mutex};

//TODO: Allow caching for N frames
//TODO: Return a kind of ResourceArc for a cached pipeline. Allow dropping after N frames pass with
// nothing request/using it
//TODO: vulkan pipeline cache object

#[derive(PartialEq, Eq, Hash)]
struct CachedGraphicsPipelineKey {
    material_pass: ResourceId,
    renderpass: ResourceId,
}

#[derive(PartialEq, Eq, Hash)]
struct CachedGraphicsPipeline {
    material_pass_resource: WeakResourceArc<MaterialPassResource>,
    renderpass_resource: WeakResourceArc<RenderPassResource>,
    graphics_pipeline: ResourceArc<GraphicsPipelineResource>,
}

#[derive(Debug)]
struct RegisteredRenderpass {
    keep_until_frame: u64,
    renderpass: WeakResourceArc<RenderPassResource>,
}

pub struct GraphicsPipelineCacheInner {
    resource_lookup_set: ResourceLookupSet,

    // index by renderphase index
    renderpass_assignments: Vec<FnvHashMap<ResourceId, RegisteredRenderpass>>,
    material_pass_assignments: Vec<FnvHashMap<ResourceId, WeakResourceArc<MaterialPassResource>>>,

    cached_pipelines: FnvHashMap<CachedGraphicsPipelineKey, CachedGraphicsPipeline>,

    current_frame_index: u64,
    frames_to_persist: u64,

    #[cfg(debug_assertions)]
    lock_call_count_previous_frame: u64,
    #[cfg(debug_assertions)]
    lock_call_count: u64,
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
        const DEFAULT_FRAMES_TO_PERSIST: u64 = 1;

        let mut renderpass_assignments = Vec::with_capacity(MAX_RENDER_PHASE_COUNT as usize);
        renderpass_assignments.resize_with(MAX_RENDER_PHASE_COUNT as usize, Default::default);

        let mut material_pass_assignments = Vec::with_capacity(MAX_RENDER_PHASE_COUNT as usize);
        material_pass_assignments.resize_with(MAX_RENDER_PHASE_COUNT as usize, Default::default);

        let inner = GraphicsPipelineCacheInner {
            resource_lookup_set,
            renderpass_assignments,
            material_pass_assignments,
            cached_pipelines: Default::default(),
            current_frame_index: 0,
            frames_to_persist: DEFAULT_FRAMES_TO_PERSIST,
            #[cfg(debug_assertions)]
            lock_call_count_previous_frame: 0,
            #[cfg(debug_assertions)]
            lock_call_count: 0,
        };

        GraphicsPipelineCache {
            render_registry: render_registry.clone(),
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn on_frame_complete(&self) {
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count_previous_frame = guard.lock_call_count + 1;
            guard.lock_call_count = 0;
        }
        guard.current_frame_index += 1;
        Self::drop_stale_pipelines(&mut *guard);
    }

    pub fn get_renderphase_by_name(
        &self,
        name: &str,
    ) -> Option<RenderPhaseIndex> {
        self.render_registry.render_phase_index_from_name(name)
    }

    // Register a renderpass as being part of a particular phase. This will a pipeline is created
    // for all appropriate renderpass/material pass pairs.
    pub fn register_renderpass_to_phase_per_frame<T: RenderPhase>(
        &self,
        renderpass: &ResourceArc<RenderPassResource>,
    ) {
        self.register_renderpass_to_phase_index_per_frame(renderpass, T::render_phase_index())
    }

    pub fn register_renderpass_to_phase_index_per_frame(
        &self,
        renderpass: &ResourceArc<RenderPassResource>,
        render_phase_index: RenderPhaseIndex,
    ) {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            inner.lock_call_count += 1;
        }

        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        if let Some(existing) = inner.renderpass_assignments[render_phase_index as usize]
            .get_mut(&renderpass.get_hash())
        {
            if existing.renderpass.upgrade().is_some() {
                existing.keep_until_frame = inner.current_frame_index + inner.frames_to_persist;
                // Nothing to do here, the previous ref is still valid
                return;
            }
        }

        inner.renderpass_assignments[render_phase_index as usize].insert(
            renderpass.get_hash(),
            RegisteredRenderpass {
                renderpass: renderpass.downgrade(),
                keep_until_frame: inner.current_frame_index + inner.frames_to_persist,
            },
        );

        //TODO: Do we need to mark this as a dirty renderpass that may need rebuilding materials?
    }

    pub fn register_material_to_phase_index(
        &self,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_phase_index: RenderPhaseIndex,
    ) {
        let mut guard = self.inner.lock().unwrap();
        #[cfg(debug_assertions)]
        {
            guard.lock_call_count += 1;
        }

        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        if let Some(existing) = guard.material_pass_assignments[render_phase_index as usize]
            .get(&material_pass.get_hash())
        {
            if existing.upgrade().is_some() {
                // Nothing to do here, the previous ref is still valid
                return;
            }
        }

        guard.material_pass_assignments[render_phase_index as usize]
            .insert(material_pass.get_hash(), material_pass.downgrade());
        //TODO: Do we need to mark this as a dirty material that may need rebuilding?
    }

    pub fn try_get_graphics_pipeline(
        &self,
        material_pass: &ResourceArc<MaterialPassResource>,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> Option<ResourceArc<GraphicsPipelineResource>> {
        // VkResult is always Ok if returning cached pipelines
        self.graphics_pipeline(material_pass, renderpass, false)
            .map(|x| x.unwrap())
    }

    pub fn get_or_create_graphics_pipeline(
        &self,
        material_pass: &ResourceArc<MaterialPassResource>,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> VkResult<ResourceArc<GraphicsPipelineResource>> {
        // graphics_pipeline never returns none if create_if_missing is true
        self.graphics_pipeline(material_pass, renderpass, true)
            .unwrap()
    }

    pub fn graphics_pipeline(
        &self,
        material_pass: &ResourceArc<MaterialPassResource>,
        renderpass: &ResourceArc<RenderPassResource>,
        create_if_missing: bool,
    ) -> Option<VkResult<ResourceArc<GraphicsPipelineResource>>> {
        let key = CachedGraphicsPipelineKey {
            material_pass: material_pass.get_hash(),
            renderpass: renderpass.get_hash(),
        };

        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            inner.lock_call_count += 1;
        }

        // Find the swapchain index for the given renderpass
        inner
            .cached_pipelines
            .get(&key)
            .map(|x| {
                debug_assert!(x.material_pass_resource.upgrade().is_some());
                debug_assert!(x.renderpass_resource.upgrade().is_some());
                Ok(x.graphics_pipeline.clone())
            })
            .or_else(|| {
                if create_if_missing {
                    let pipeline = inner
                        .resource_lookup_set
                        .get_or_create_graphics_pipeline(&material_pass, &renderpass);

                    if let Ok(pipeline) = pipeline {
                        inner.cached_pipelines.insert(
                            key,
                            CachedGraphicsPipeline {
                                graphics_pipeline: pipeline.clone(),
                                renderpass_resource: renderpass.downgrade(),
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

    pub fn precache_pipelines_for_all_phases(&self) -> VkResult<()> {
        let mut guard = self.inner.lock().unwrap();
        let inner = &mut *guard;
        #[cfg(debug_assertions)]
        {
            inner.lock_call_count += 1;
        }

        //TODO: Avoid iterating everything all the time
        for render_phase_index in 0..MAX_RENDER_PHASE_COUNT {
            for (renderpass_hash, renderpass) in
                &inner.renderpass_assignments[render_phase_index as usize]
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

        Ok(())
    }

    fn drop_stale_pipelines(inner: &mut GraphicsPipelineCacheInner) {
        let current_frame_index = inner.current_frame_index;
        for phase in &mut inner.renderpass_assignments {
            phase.retain(|_k, v| {
                v.renderpass.upgrade().is_some() && v.keep_until_frame > current_frame_index
            });
        }

        for phase in &mut inner.material_pass_assignments {
            phase.retain(|_k, v| v.upgrade().is_some());
        }

        inner.cached_pipelines.retain(|_k, v| {
            let renderpass_still_exists = v.renderpass_resource.upgrade().is_some();
            let material_pass_still_exists = v.material_pass_resource.upgrade().is_some();

            if !renderpass_still_exists || !material_pass_still_exists {
                log::trace!("Dropping pipeline, renderpass_still_exists: {}, material_pass_still_exists: {}", renderpass_still_exists, material_pass_still_exists);
            }

            renderpass_still_exists && material_pass_still_exists
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
