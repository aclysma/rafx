use renderer::nodes::{RenderPhaseIndex, SubmitNode};
use renderer::nodes::RenderPhase;

renderer::declare_render_phase!(
    ShadowMapRenderPhase,
    SHADOW_MAP_RENDER_PHASE_INDEX,
    shadow_map_render_phase_sort_submit_nodes
);

fn shadow_map_render_phase_sort_submit_nodes(mut submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
    // Sort by feature
    log::trace!(
        "Sort phase {}",
        ShadowMapRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));

    submit_nodes
}
