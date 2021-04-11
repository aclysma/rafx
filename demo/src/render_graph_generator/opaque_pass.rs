use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase};
use rafx::graph::*;

use super::RenderGraphContext;
use super::ShadowMapImageResources;
use rafx::api::RafxColorClearValue;
use rafx::nodes::RenderJobWriteContext;

pub(super) struct OpaquePass {
    pub(super) node: RenderGraphNodeId,
    pub(super) color: RenderGraphImageUsageId,
    pub(super) shadow_maps: Vec<RenderGraphImageUsageId>,
}

pub(super) fn opaque_pass(
    context: &mut RenderGraphContext,
    depth_prepass: RenderGraphImageUsageId,
    shadow_map_passes: &[ShadowMapImageResources],
) -> OpaquePass {
    let node = context
        .graph
        .add_node("Opaque", RenderGraphQueue::DefaultGraphics);

    let color = context.graph.create_color_attachment(
        node,
        0,
        Some(RafxColorClearValue([0.0, 0.0, 0.0, 0.0])),
        RenderGraphImageConstraint {
            samples: Some(context.graph_config.samples),
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
        Default::default(),
    );
    context.graph.set_image_name(color, "color");

    context.graph.read_depth_attachment(
        node,
        depth_prepass,
        RenderGraphImageConstraint {
            samples: Some(context.graph_config.samples),
            format: Some(context.graph_config.depth_format),
            ..Default::default()
        },
        Default::default(),
    );

    let mut shadow_maps = Vec::with_capacity(shadow_map_passes.len());
    for shadow_map_pass in shadow_map_passes {
        let sampled_image = match shadow_map_pass {
            ShadowMapImageResources::Single(image) => {
                context
                    .graph
                    .sample_image(node, *image, Default::default(), Default::default())
            }
            ShadowMapImageResources::Cube(cube_map_image) => context.graph.sample_image(
                node,
                *cube_map_image,
                Default::default(),
                Default::default(),
            ),
        };
        shadow_maps.push(sampled_image);
    }

    context
        .graph
        .add_render_phase_dependency::<OpaqueRenderPhase>(node);

    let main_view = context.main_view.clone();

    context.graph.set_renderpass_callback(node, move |args| {
        let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);

        args.graph_context
            .prepared_render_data()
            .write_view_phase::<OpaqueRenderPhase>(&main_view, &mut write_context)?;

        args.graph_context
            .prepared_render_data()
            .write_view_phase::<TransparentRenderPhase>(&main_view, &mut write_context)
    });

    OpaquePass {
        node,
        color,
        shadow_maps,
    }
}
