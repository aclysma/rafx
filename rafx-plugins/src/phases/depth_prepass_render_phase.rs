use rafx::render_features::RenderFeatureSubmitNode;

rafx::declare_render_phase!(
    DepthPrepassRenderPhase,
    DEPTH_PREPASS_RENDER_PHASE_INDEX,
    depth_prepass_render_phase_sort_submit_nodes
);

#[profiling::function]
fn depth_prepass_render_phase_sort_submit_nodes(_submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sorting is unnecessary because depth-only "overdraw" is not a performance concern
}
