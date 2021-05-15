use rafx::render_features::RenderPhase;
use rafx::render_features::{RenderFeatureSubmitNode, RenderPhaseIndex};

//
// A phase combines renderables that may come from different features. This example doesnt't use
// render nodes fully, but the pipeline cache uses it to define which renderpass/material pairs
//

rafx::declare_render_phase!(
    OpaqueRenderPhase,
    OPAQUE_RENDER_PHASE_INDEX,
    opaque_render_phase_sort_submit_nodes
);

#[profiling::function]
fn opaque_render_phase_sort_submit_nodes(submit_nodes: &mut Vec<RenderFeatureSubmitNode>) {
    // Sort by feature
    log::trace!(
        "Sort phase {}",
        OpaqueRenderPhase::render_phase_debug_name()
    );
    submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));
}
