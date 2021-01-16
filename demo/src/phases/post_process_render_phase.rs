use rafx::nodes::RenderPhase;
use rafx::nodes::{RenderPhaseIndex, SubmitNode};

rafx::declare_render_phase!(
    PostProcessRenderPhase,
    POST_PROCESS_RENDER_PHASE_INDEX,
    post_process_render_phase_sort_submit_nodes
);

#[profiling::function]
fn post_process_render_phase_sort_submit_nodes(submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
    // This render phase doesn't submit nodes and does not need sorting. It exists so that materials
    // and render target metas can be associated with it in the pipeline cache. This keeps pipelines
    // loaded and available across frames, and allows new materials to be built during the asset
    // load instead of on the render code path
    assert!(submit_nodes.is_empty());

    submit_nodes
}
