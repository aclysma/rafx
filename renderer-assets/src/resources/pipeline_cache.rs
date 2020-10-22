use crate::vk_description::SwapchainSurfaceInfo;
use crate::{
    vk_description as def, ResourceLookupSet, RenderPassResource, ResourceArc,
    GraphicsPipelineResource, MaterialPassResource,
};
use atelier_assets::loader::handle::Handle;
use crate::assets::RenderpassAsset;
use renderer_nodes::{RenderPhase, RenderPhaseIndex, MAX_RENDER_PHASE_COUNT, RenderRegistry};
use crate::resources::resource_arc::{WeakResourceArc, ResourceId};
use fnv::{FnvHashMap, FnvHashSet};
use std::hash::Hash;
use crate::resources::resource_lookup::GraphicsPipelineKey;

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

struct RegisteredRenderpass {
    keep_until_frame: u64,
    renderpass: WeakResourceArc<RenderPassResource>,
}

pub struct GraphicsPipelineCache {
    render_registry: RenderRegistry,

    // index by renderphase index
    renderpass_assignments: Vec<FnvHashMap<ResourceId, RegisteredRenderpass>>,
    material_pass_assignments: Vec<FnvHashMap<ResourceId, WeakResourceArc<MaterialPassResource>>>,

    cached_pipelines: FnvHashMap<CachedGraphicsPipelineKey, CachedGraphicsPipeline>,

    current_frame_index: u64,
    frames_to_persist: u64,
}

impl GraphicsPipelineCache {
    pub fn new(render_registry: &RenderRegistry) -> Self {
        const DEFAULT_FRAMES_TO_PERSIST: u64 = 3;

        let mut renderpass_assignments = Vec::with_capacity(MAX_RENDER_PHASE_COUNT as usize);
        renderpass_assignments.resize_with(MAX_RENDER_PHASE_COUNT as usize, || Default::default());

        let mut material_pass_assignments = Vec::with_capacity(MAX_RENDER_PHASE_COUNT as usize);
        material_pass_assignments
            .resize_with(MAX_RENDER_PHASE_COUNT as usize, || Default::default());

        GraphicsPipelineCache {
            render_registry: render_registry.clone(),
            renderpass_assignments,
            material_pass_assignments,
            cached_pipelines: Default::default(),
            current_frame_index: 0,
            frames_to_persist: DEFAULT_FRAMES_TO_PERSIST,
        }
    }

    pub fn on_frame_complete(&mut self) {
        self.current_frame_index += 1;
        self.drop_unused_pipelines();
    }

    // Call to assign a string name to a renderphase. This is permanent and multiple names can alias
    // to the same renderphase
    // fn add_phase_name<T: RenderPhase>(&mut self, name: String) {
    //     let old = self.phase_name_to_index.insert(name, T::render_phase_index());
    //     assert!(old.is_none());
    // }

    pub fn get_renderphase_by_name(
        &self,
        name: &str,
    ) -> Option<RenderPhaseIndex> {
        self.render_registry.render_phase_index_from_name(name)
    }

    // Register a renderpass as being part of a particular phase. This will a pipeline is created
    // for all appropriate renderpass/material pass pairs.
    pub fn per_frame_register_renderpass_to_phase<T: RenderPhase>(
        &mut self,
        renderpass: &ResourceArc<RenderPassResource>,
    ) {
        self.per_frame_register_renderpass_to_phase_index(renderpass, T::render_phase_index())
    }

    pub fn per_frame_register_renderpass_to_phase_index(
        &mut self,
        renderpass: &ResourceArc<RenderPassResource>,
        render_phase_index: RenderPhaseIndex,
    ) {
        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        if let Some(existing) =
            self.renderpass_assignments[render_phase_index as usize].get_mut(&renderpass.get_hash())
        {
            if existing.renderpass.upgrade().is_some() {
                existing.keep_until_frame = self.current_frame_index + self.frames_to_persist;
                // Nothing to do here, the previous ref is still valid
                return;
            }
        }

        self.renderpass_assignments[render_phase_index as usize].insert(
            renderpass.get_hash(),
            RegisteredRenderpass {
                renderpass: renderpass.downgrade(),
                keep_until_frame: self.current_frame_index + self.frames_to_persist,
            },
        );

        //TODO: Do we need to mark this as a dirty renderpass that may need rebuilding materials?
    }

    pub fn register_material_to_phase_index(
        &mut self,
        material_pass: &ResourceArc<MaterialPassResource>,
        render_phase_index: RenderPhaseIndex,
    ) {
        assert!(render_phase_index < MAX_RENDER_PHASE_COUNT);
        if let Some(existing) = self.material_pass_assignments[render_phase_index as usize]
            .get(&material_pass.get_hash())
        {
            if existing.upgrade().is_some() {
                // Nothing to do here, the previous ref is still valid
                return;
            }
        }

        self.material_pass_assignments[render_phase_index as usize]
            .insert(material_pass.get_hash(), material_pass.downgrade());
        //TODO: Do we need to mark this as a dirty material that may need rebuilding?
    }

    //TODO: OR do it by material/swapchain info?
    pub fn find_graphics_pipeline(
        &self,
        material: &ResourceArc<MaterialPassResource>,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> Option<ResourceArc<GraphicsPipelineResource>> {
        let key = CachedGraphicsPipelineKey {
            material_pass: material.get_hash(),
            renderpass: renderpass.get_hash(),
        };

        // Find the swapchain index for the given renderpass
        self.cached_pipelines.get(&key).map(|x| {
            debug_assert!(x.material_pass_resource.upgrade().is_some());
            debug_assert!(x.renderpass_resource.upgrade().is_some());
            x.graphics_pipeline.clone()
        })
    }

    pub fn drop_unused_pipelines(&mut self) {
        let current_frame_index = self.current_frame_index;
        for phase in &mut self.renderpass_assignments {
            phase.retain(|k, v| {
                v.renderpass.upgrade().is_some() && v.keep_until_frame > current_frame_index
            });
        }

        for phase in &mut self.material_pass_assignments {
            phase.retain(|k, v| v.upgrade().is_some());
        }

        self.cached_pipelines.retain(|k, v| {
            v.renderpass_resource.upgrade().is_some()
                && v.material_pass_resource.upgrade().is_some()
        })
    }
}
