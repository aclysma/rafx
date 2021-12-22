use rafx::render_features::RenderFeatureSubmitNode;

rafx::declare_render_phase!(
    DebugPipRenderPhase,
    DEBUG_PIP_RENDER_PHASE_INDEX,
    debug_pip_phase_sort_submit_nodes
);

#[profiling::function]
fn debug_pip_phase_sort_submit_nodes(_submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sorting is unnecessary because there is one feature that will produce one submit node
}
