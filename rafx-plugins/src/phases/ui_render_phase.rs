use rafx::render_features::RenderPhase;
use rafx::render_features::{RenderFeatureSubmitNode, RenderPhaseIndex};

rafx::declare_render_phase!(
    UiRenderPhase,
    UI_RENDER_PHASE_INDEX,
    ui_render_phase_sort_submit_nodes
);

#[profiling::function]
fn ui_render_phase_sort_submit_nodes(submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sort by feature
    log::trace!("Sort phase {}", UiRenderPhase::render_phase_debug_name());
    submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));
}
