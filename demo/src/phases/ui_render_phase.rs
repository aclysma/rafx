use renderer::nodes::RenderPhase;
use renderer::nodes::{RenderPhaseIndex, SubmitNode};

renderer::declare_render_phase!(
    UiRenderPhase,
    UI_RENDER_PHASE_INDEX,
    ui_render_phase_sort_submit_nodes
);

fn ui_render_phase_sort_submit_nodes(mut submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
    // Sort by feature
    log::trace!("Sort phase {}", UiRenderPhase::render_phase_debug_name());
    submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));

    submit_nodes
}
