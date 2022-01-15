use crate::phases::DebugPipRenderPhase;
use rafx::graph::*;

use super::BasicPipelineContext;
use crate::features::debug_pip::DebugPipRenderResource;

pub(super) struct DebugPipPass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) color: RenderGraphImageUsageId,
}

pub(super) fn debug_pip_pass(
    context: &mut BasicPipelineContext,
    previous_pass_color: RenderGraphImageUsageId,
) -> DebugPipPass {
    // This node has a single color attachment
    let node = context
        .graph
        .add_node("DebugPip", RenderGraphQueue::DefaultGraphics);
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
        .add_render_phase_dependency::<DebugPipRenderPhase>(node);

    let mut debug_pip_render_resource = context
        .render_resources
        .fetch_mut::<DebugPipRenderResource>();
    let sampled_render_graph_images: Vec<_> = debug_pip_render_resource
        .render_graph_images()
        .iter()
        .map(|&x| {
            context.graph.sample_image(
                node,
                x,
                Default::default(),
                RenderGraphImageViewOptions::default(),
            )
        })
        .collect();
    debug_pip_render_resource.set_sampled_render_graph_images(sampled_render_graph_images);

    // When the node is executed, we automatically set up the renderpass/framebuffer/command
    // buffer. Just add the draw calls.
    let main_view = context.main_view.clone();
    context.graph.set_renderpass_callback(node, move |args| {
        profiling::scope!("DebugPip Pass");
        // Kick the material system to emit all draw calls for the DebugPipRenderPhase for the view
        args.write_view_phase::<DebugPipRenderPhase>(&main_view)
    });

    DebugPipPass { node, color }
}
