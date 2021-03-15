use rafx::nodes::RenderPhase;
use rafx::nodes::{RenderPhaseIndex, SubmitNode};

rafx::declare_render_phase!(
    DepthPrepassRenderPhase,
    DEPTH_PREPASS_RENDER_PHASE_INDEX,
    depth_prepass_render_phase_sort_submit_nodes
);

#[profiling::function]
fn depth_prepass_render_phase_sort_submit_nodes(
    mut submit_nodes: Vec<SubmitNode>
) -> Vec<SubmitNode> {
    // Sort by distance from camera front to back
    log::trace!(
        "Sort phase {}",
        DepthPrepassRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| a.distance().partial_cmp(&b.distance()).unwrap());

    submit_nodes
}
