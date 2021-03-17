use rafx::nodes::RenderPhase;
use rafx::nodes::{RenderPhaseIndex, SubmitNode};

rafx::declare_render_phase!(
    OpaqueRenderPhase,
    OPAQUE_RENDER_PHASE_INDEX,
    opaque_render_phase_sort_submit_nodes
);

#[profiling::function]
fn opaque_render_phase_sort_submit_nodes(submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
    // Sorting is unnecessary because of the depth pre-pass.
    submit_nodes
}
