use rafx::render_features::RenderFeatureSubmitNode;

rafx::declare_render_phase!(
    ShadowMapRenderPhase,
    SHADOW_MAP_RENDER_PHASE_INDEX,
    shadow_map_render_phase_sort_submit_nodes
);

#[profiling::function]
fn shadow_map_render_phase_sort_submit_nodes(_submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sorting is unnecessary because depth-only "overdraw" is not a performance concern
}
