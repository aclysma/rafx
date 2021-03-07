use crate::phases::UiRenderPhase;
use rafx::graph::*;

use super::RenderGraphContext;
use rafx::nodes::RenderJobWriteContext;

pub(super) struct UiPass {
    pub(super) node: RenderGraphNodeId,
    pub(super) color: RenderGraphImageUsageId,
}

pub(super) fn ui_pass(
    context: &mut RenderGraphContext,
    previous_pass_color: RenderGraphImageUsageId,
) -> UiPass {
    // This node has a single color attachment
    let node = context
        .graph
        .add_node("Ui", RenderGraphQueue::DefaultGraphics);
    let color = context.graph.modify_color_attachment(
        node,
        previous_pass_color,
        0,
        None,
        Default::default(),
        Default::default(),
    );
    context.graph.set_image_name(color, "color");

    // Adding a phase dependency insures that we create all the pipelines for materials
    // associated with the phase. This controls how long we keep the pipelines allocated and
    // allows us to precache pipelines for materials as they are loaded
    context
        .graph
        .add_render_phase_dependency::<UiRenderPhase>(node);

    // When the node is executed, we automatically set up the renderpass/framebuffer/command
    // buffer. Just add the draw calls.
    let main_view = context.main_view.clone();
    context.graph.set_renderpass_callback(node, move |args| {
        // Kick the material system to emit all draw calls for the UiRenderPhase for the view
        let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
        args.graph_context
            .prepared_render_data()
            .write_view_phase::<UiRenderPhase>(&main_view, &mut write_context)
    });

    UiPass { node, color }
}
