use rafx::render_features::RenderPhase;
use rafx::render_features::{RenderFeatureSubmitNode, RenderPhaseIndex};

rafx::declare_render_phase!(
    WireframeRenderPhase,
    WIREFRAME_RENDER_PHASE_INDEX,
    wireframe_render_phase_sort_submit_nodes
);

#[profiling::function]
fn wireframe_render_phase_sort_submit_nodes(_submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sorting is unnecessary because of the depth pre-pass.
}
