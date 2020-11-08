use renderer::nodes::RenderPhase;
use renderer::nodes::{RenderPhaseIndex, SubmitNode};

renderer::declare_render_phase!(
    DemoTransparentRenderPhase,
    TRANSPARENT_RENDER_PHASE_INDEX,
    demo_transparent_render_phase_sort_submit_nodes
);

fn demo_transparent_render_phase_sort_submit_nodes(
    mut submit_nodes: Vec<SubmitNode>
) -> Vec<SubmitNode> {
    // Sort by distance from camera back to front
    log::trace!(
        "Sort phase {}",
        DemoTransparentRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| b.distance().partial_cmp(&a.distance()).unwrap());

    submit_nodes
}
