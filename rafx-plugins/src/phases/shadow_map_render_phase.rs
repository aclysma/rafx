use rafx::render_features::RenderFeatureSubmitNode;
use rafx::render_features::RenderPhase;

rafx::declare_render_phase!(
    ShadowMapRenderPhase,
    SHADOW_MAP_RENDER_PHASE_INDEX,
    shadow_map_render_phase_sort_submit_nodes
);

#[profiling::function]
fn shadow_map_render_phase_sort_submit_nodes(submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sort by distance from camera front to back
    log::trace!(
        "Sort phase {}",
        ShadowMapRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| a.distance().partial_cmp(&b.distance()).unwrap());
}
