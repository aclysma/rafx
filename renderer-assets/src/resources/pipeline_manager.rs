use crate::vk_description::SwapchainSurfaceInfo;
use crate::{
    vk_description as def, ResourceLookupSet, RenderPassResource, ResourceArc,
    GraphicsPipelineResource, MaterialPassResource,
};
use atelier_assets::loader::handle::Handle;
use crate::assets::RenderpassAsset;
use renderer_nodes::{RenderPhase, RenderPhaseIndex};
use crate::resources::resource_arc::WeakResourceArc;
use fnv::FnvHashMap;
use std::hash::Hash;

//TODO:
// - Fix the rendergraph to persist renderpasses and framebuffers across frames
// - Update this class to create pipelines
//
// - OPTIONAL: Add method to immediately delete resources instead of waiting for them to drop after
//   N frames. This would be useful for the persist-across-frames cache.

struct PhaseRenderpassAssignment {
    renderphase_index: RenderPhaseIndex,
    renderpasses: Vec<WeakResourceArc<RenderPassResource>>,
}

struct PipelineCompiler {
    phase_renderpass_assignments: FnvHashMap<PhaseRenderpassAssignment, ()>,
}

impl PipelineCompiler {
    fn add_renderpass_to_phase<T: RenderPhase>(
        &mut self,
        renderpass: ResourceArc<RenderPassResource>,
    ) {
    }

    // fn get_pipeline<T: RenderPhase>(
    //     &self,
    //     resources: &mut ResourceLookupSet,
    //     material: &ResourceArc<MaterialPassResource>,
    //     renderpass: &ResourceArc<RenderPassResource>
    // ) -> ResourceArc<GraphicsPipelineResource> {
    //     resources.get_or_create_graphics_pipeline(material, renderpass)
    // }

    fn update(&mut self) {
        // Clear any stale renderpass assignments
    }
}
