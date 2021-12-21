use super::RenderGraphContext;
use crate::phases::DepthPrepassRenderPhase;
use rafx::api::RafxDepthStencilClearValue;
use rafx::graph::*;
use rafx::render_features::RenderJobCommandBufferContext;

pub(super) struct DepthPrepass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) depth: RenderGraphImageUsageId,
}

pub(super) fn depth_prepass(context: &mut RenderGraphContext) -> Option<DepthPrepass> {
    if !context
        .main_view
        .phase_is_relevant::<DepthPrepassRenderPhase>()
    {
        return None;
    }

    let node = context
        .graph
        .add_node("DepthPrepass", RenderGraphQueue::DefaultGraphics);

    let depth = context.graph.create_depth_attachment(
        node,
        Some(RafxDepthStencilClearValue {
            depth: 0.0,
            stencil: 0,
        }),
        RenderGraphImageConstraint {
            samples: Some(context.graph_config.samples),
            format: Some(context.graph_config.depth_format),
            ..Default::default()
        },
        Default::default(),
    );
    context.graph.set_image_name(depth, "depth");

    context
        .graph
        .add_render_phase_dependency::<DepthPrepassRenderPhase>(node);

    let main_view = context.main_view.clone();

    context.graph.set_renderpass_callback(node, move |args| {
        profiling::scope!("Depth Prepass");
        let mut write_context =
            RenderJobCommandBufferContext::from_graph_visit_render_pass_args(&args);
        args.graph_context
            .prepared_render_data()
            .write_view_phase::<DepthPrepassRenderPhase>(&main_view, &mut write_context)
    });

    Some(DepthPrepass { node, depth })
}
