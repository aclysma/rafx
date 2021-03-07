use super::RenderGraphContext;
use crate::features::mesh::shadow_map_resource::ShadowMapResource;
use crate::features::mesh::ShadowMapRenderView;
use crate::phases::ShadowMapRenderPhase;
use rafx::api::{RafxDepthStencilClearValue, RafxResourceType};
use rafx::graph::*;
use rafx::nodes::{RenderJobWriteContext, RenderView};

pub(super) struct ShadowMapPass {
    pub(super) node: RenderGraphNodeId,
    pub(super) depth: RenderGraphImageUsageId,
}

pub(super) enum ShadowMapImageResources {
    Single(RenderGraphImageUsageId),
    Cube(RenderGraphImageUsageId),
}

pub(super) fn shadow_map_passes(context: &mut RenderGraphContext) -> Vec<ShadowMapImageResources> {
    let mut shadow_map_resource = context.render_resources.fetch_mut::<ShadowMapResource>();
    let shadow_map_views = shadow_map_resource.shadow_map_render_views();

    let mut shadow_map_passes = Vec::default();
    for shadow_map_view in shadow_map_views {
        match shadow_map_view {
            ShadowMapRenderView::Single(render_view) => {
                let shadow_map_node = context
                    .graph
                    .add_node("create shadowmap", RenderGraphQueue::DefaultGraphics);
                let depth_image = context.graph.create_unattached_image(
                    shadow_map_node,
                    RenderGraphImageConstraint {
                        format: Some(context.graph_config.depth_format),
                        extents: Some(RenderGraphImageExtents::Custom(
                            render_view.extents_width(),
                            render_view.extents_height(),
                            1,
                        )),
                        ..Default::default()
                    },
                    Default::default(),
                );

                let shadow_map_pass = shadow_map_pass(context, render_view, depth_image, 0);
                shadow_map_passes.push(ShadowMapImageResources::Single(shadow_map_pass.depth));
            }
            ShadowMapRenderView::Cube(render_view) => {
                let cube_map_node = context
                    .graph
                    .add_node("create cube shadowmap", RenderGraphQueue::DefaultGraphics);

                let cube_map_xy_size = 1024;

                let mut cube_map_image = context.graph.create_unattached_image(
                    cube_map_node,
                    RenderGraphImageConstraint {
                        format: Some(context.graph_config.depth_format),
                        layer_count: Some(6),
                        extents: Some(RenderGraphImageExtents::Custom(
                            cube_map_xy_size,
                            cube_map_xy_size,
                            1,
                        )),
                        resource_type: RafxResourceType::TEXTURE_CUBE
                            | RafxResourceType::RENDER_TARGET_ARRAY_SLICES,
                        ..Default::default()
                    },
                    Default::default(),
                );

                for i in 0..6 {
                    cube_map_image =
                        shadow_map_pass(context, &render_view[i], cube_map_image, i).depth;
                }

                shadow_map_passes.push(ShadowMapImageResources::Cube(cube_map_image));
            }
        }
    }

    let mut usage_ids = Vec::default();
    for pass in &shadow_map_passes {
        match pass {
            ShadowMapImageResources::Single(usage_id) => usage_ids.push(*usage_id),
            ShadowMapImageResources::Cube(usage_id) => usage_ids.push(*usage_id),
        }
    }
    shadow_map_resource.set_shadow_map_image_usage_ids(usage_ids);

    shadow_map_passes
}

fn shadow_map_pass(
    context: &mut RenderGraphContext,
    render_view: &RenderView,
    depth_image: RenderGraphImageUsageId,
    layer: usize,
) -> ShadowMapPass {
    let node = context
        .graph
        .add_node("Shadow", RenderGraphQueue::DefaultGraphics);

    let depth = context.graph.modify_depth_attachment(
        node,
        depth_image,
        Some(RafxDepthStencilClearValue {
            depth: 0.0,
            stencil: 0,
        }),
        RenderGraphImageConstraint::default(),
        RenderGraphImageViewOptions::array_slice(layer as u16),
    );
    context.graph.set_image_name(depth, "depth");

    context
        .graph
        .add_render_phase_dependency::<ShadowMapRenderPhase>(node);

    let render_view = render_view.clone();
    context.graph.set_renderpass_callback(node, move |args| {
        let mut write_context = RenderJobWriteContext::from_graph_visit_render_pass_args(&args);
        args.graph_context
            .prepared_render_data()
            .write_view_phase::<ShadowMapRenderPhase>(&render_view, &mut write_context)
    });

    ShadowMapPass { node, depth }
}
