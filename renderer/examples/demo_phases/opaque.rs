use renderer::nodes::{RenderPhaseIndex, SubmitNode};
use renderer::nodes::RenderPhase;
use std::convert::TryInto;

renderer::declare_render_phase!(
    DemoOpaqueRenderPhase,
    OPAQUE_RENDER_PHASE_INDEX,
    demo_opaque_render_phase_sort_submit_nodes
);

fn demo_opaque_render_phase_sort_submit_nodes(
    mut submit_nodes: Vec<SubmitNode>
) -> Vec<SubmitNode> {
    // Sort by feature
    log::trace!(
        "Sort phase {}",
        DemoOpaqueRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));

    submit_nodes
}
