use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase, WireframeRenderPhase};
use rafx::graph::*;

use super::depth_prepass::DepthPrepass;
use super::RenderGraphContext;
use crate::pipelines::modern::graph_generator::light_binning::LightBuildListsPass;
use crate::pipelines::modern::graph_generator::shadow_map_pass::ShadowMapPassOutput;
use rafx::api::{RafxColorClearValue, RafxDepthStencilClearValue};
use rafx::render_features::RenderJobCommandBufferContext;

pub(super) struct OpaquePass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) color: RenderGraphImageUsageId,
    #[allow(dead_code)]
    pub(super) shadow_map_atlas: RenderGraphImageUsageId,
}

pub(super) fn opaque_pass(
    context: &mut RenderGraphContext,
    depth_prepass: Option<DepthPrepass>,
    shadow_map_pass_output: &ShadowMapPassOutput,
    light_build_lists_pass: &LightBuildListsPass,
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

    //let mut shadow_maps = Vec::with_capacity(shadow_map_passes.len());

    if context.graph_config.show_surfaces && depth_prepass.is_some() {
        context.graph.read_depth_attachment(
            node,
            depth_prepass.unwrap().depth,
            RenderGraphImageConstraint {
                samples: Some(context.graph_config.samples),
                format: Some(context.graph_config.depth_format),
                ..Default::default()
            },
            Default::default(),
        );
    } else {
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
    }

    // This is a buffer owned by MeshAdvLightBinRenderResource
    context.graph.read_storage_buffer(
        node,
        light_build_lists_pass.light_lists_buffer,
        Default::default(),
    );

    let shadow_map_atlas = context.graph.sample_image(
        node,
        shadow_map_pass_output.shadow_atlas_image,
        Default::default(),
        Default::default(),
    );

    context
        .graph
        .add_render_phase_dependency::<OpaqueRenderPhase>(node);

    let main_view = context.main_view.clone();
    let show_models = context.graph_config.show_surfaces;

    context.graph.set_renderpass_callback(node, move |args| {
        let mut write_context =
            RenderJobCommandBufferContext::from_graph_visit_render_pass_args(&args);

        if show_models {
            {
                profiling::scope!("Opaque Pass");
                args.graph_context
                    .prepared_render_data()
                    .write_view_phase::<OpaqueRenderPhase>(&main_view, &mut write_context)?;
            }

            {
                profiling::scope!("Transparent Pass");
                args.graph_context
                    .prepared_render_data()
                    .write_view_phase::<TransparentRenderPhase>(&main_view, &mut write_context)?;
            }
        }

        {
            profiling::scope!("Wireframes Pass");
            args.graph_context
                .prepared_render_data()
                .write_view_phase::<WireframeRenderPhase>(&main_view, &mut write_context)?;
        }

        Ok(())
    });

    OpaquePass {
        node,
        color,
        shadow_map_atlas,
    }
}
