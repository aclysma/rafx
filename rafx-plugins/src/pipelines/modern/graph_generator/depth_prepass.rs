use super::ModernPipelineContext;
use crate::phases::DepthPrepassRenderPhase;
use rafx::api::{RafxColorClearValue, RafxDepthStencilClearValue, RafxFormat};
use rafx::graph::*;
use rafx::render_features::RenderJobCommandBufferContext;

pub(super) struct DepthPrepass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) velocity_rt: RenderGraphImageUsageId,
    pub(super) depth: RenderGraphImageUsageId,
}

pub(super) fn depth_prepass(context: &mut ModernPipelineContext) -> DepthPrepass {
    let node = context
        .graph
        .add_renderpass_node("DepthPrepass", RenderGraphQueue::DefaultGraphics);

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

    let velocity_rt = context.graph.create_color_attachment(
        node,
        0,
        // Large number means reproject using camera matrices
        Some(RafxColorClearValue([9999999.0, 9999999.0, 0.0, 0.0])),
        RenderGraphImageConstraint {
            samples: Some(context.graph_config.samples),
            format: Some(RafxFormat::R32G32_SFLOAT),
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

    DepthPrepass {
        node,
        velocity_rt,
        depth,
    }
}
