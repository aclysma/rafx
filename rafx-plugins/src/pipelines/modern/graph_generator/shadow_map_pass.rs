use super::ModernPipelineContext;
use crate::features::mesh_adv::{
    MeshAdvShadowMapResource, MeshAdvStaticResources, ShadowMapAtlasClearTileVertex,
    SHADOW_MAP_ATLAS_CLEAR_TILE_LAYOUT,
};
use crate::phases::ShadowMapRenderPhase;
use rafx::api::{
    RafxBufferDef, RafxDepthStencilClearValue, RafxMemoryUsage, RafxResourceType,
    RafxVertexBufferBinding,
};
use rafx::framework::render_features::RenderPhase;
use rafx::graph::*;
use rafx::render_features::{RenderJobCommandBufferContext, RenderView};

pub(super) struct ShadowMapPassOutput {
    pub(super) shadow_atlas_image: RenderGraphImageUsageId,
}

pub(super) fn shadow_map_passes(
    context: &mut ModernPipelineContext,
    shadow_atlas_image: RenderGraphExternalImageId,
    shadow_atlas_needs_full_clear: bool,
) -> ShadowMapPassOutput {
    let shadow_map_resource = context.render_resources.fetch::<MeshAdvShadowMapResource>();

    //
    // Early out if we don't need to update the shadow map
    //
    let mut shadow_atlas_usage = context.graph.read_external_image(shadow_atlas_image);
    if shadow_map_resource.shadow_maps_needing_redraw().is_empty() {
        //let mut debug_pip_resource = context
        //    .render_resources
        //    .fetch_mut::<DebugPipRenderResource>()
        //    .add_render_graph_image(shadow_atlas_usage);

        return ShadowMapPassOutput {
            shadow_atlas_image: shadow_atlas_usage,
        };
    }

    let mesh_static_resources = context.render_resources.fetch::<MeshAdvStaticResources>();
    let clear_tiles_material = context
        .asset_manager
        .committed_asset(&mesh_static_resources.shadow_map_atlas_clear_tiles_material)
        .unwrap()
        .get_single_material_pass()
        .unwrap();

    let render_views_to_draw: Vec<RenderView> = shadow_map_resource
        .shadow_map_render_views()
        .iter()
        .filter_map(|x| x.clone())
        .collect();

    let node = context
        .graph
        .add_node("Shadow", RenderGraphQueue::DefaultGraphics);

    let clear_value = if shadow_atlas_needs_full_clear {
        Some(RafxDepthStencilClearValue {
            depth: 0.0,
            stencil: 0,
        })
    } else {
        None
    };

    shadow_atlas_usage = context.graph.modify_depth_attachment(
        node,
        shadow_atlas_usage,
        clear_value,
        RenderGraphImageConstraint::default(),
        RenderGraphImageViewOptions::default(),
    );
    context.graph.set_image_name(shadow_atlas_usage, "depth");

    context
        .graph
        .add_render_phase_dependency::<ShadowMapRenderPhase>(node);

    context.graph.set_renderpass_callback(node, move |args| {
        profiling::scope!("Shadow Map Pass");

        // We can skip clearing individual maps if we cleared ALL the maps
        if !shadow_atlas_needs_full_clear {
            let shadow_map_resource = args
                .graph_context
                .render_resources()
                .fetch::<MeshAdvShadowMapResource>();
            let vertex_count = render_views_to_draw.len() as u64 * 6;

            //
            // Create a vertex buffer with tiles to clear
            //
            let vertex_buffer = {
                let device_context = args.graph_context.device_context();
                let dyn_resource_allocator_set = args
                    .graph_context
                    .resource_context()
                    .create_dyn_resource_allocator_set();

                let vertex_buffer_size =
                    vertex_count * std::mem::size_of::<ShadowMapAtlasClearTileVertex>() as u64;

                let vertex_buffer = device_context
                    .create_buffer(&RafxBufferDef {
                        size: vertex_buffer_size,
                        memory_usage: RafxMemoryUsage::CpuToGpu,
                        resource_type: RafxResourceType::VERTEX_BUFFER,
                        ..Default::default()
                    })
                    .unwrap();

                let mut data = Vec::with_capacity(vertex_count as usize);
                for &shadow_view_index in shadow_map_resource.shadow_maps_needing_redraw() {
                    let info = shadow_map_resource
                        .shadow_map_atlas_element_info_for_shadow_view_index(shadow_view_index);

                    // These are UV coordinates so Y is positive going down
                    // Top Left
                    let tl = ShadowMapAtlasClearTileVertex {
                        position: [info.uv_min.x, info.uv_min.y],
                    };

                    // Top Right
                    let tr = ShadowMapAtlasClearTileVertex {
                        position: [info.uv_max.x, info.uv_min.y],
                    };

                    // Bottom Left
                    let bl = ShadowMapAtlasClearTileVertex {
                        position: [info.uv_min.x, info.uv_max.y],
                    };

                    // Bottom Right
                    let br = ShadowMapAtlasClearTileVertex {
                        position: [info.uv_max.x, info.uv_max.y],
                    };

                    data.push(tr);
                    data.push(tl);
                    data.push(br);
                    data.push(br);
                    data.push(tl);
                    data.push(bl);
                }

                vertex_buffer
                    .copy_to_host_visible_buffer(data.as_slice())
                    .unwrap();

                dyn_resource_allocator_set.insert_buffer(vertex_buffer)
            };

            //
            // Get the pipeline for clearing the tiles
            //
            let pipeline = args
                .graph_context
                .resource_context()
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    Some(ShadowMapRenderPhase::render_phase_index()),
                    &clear_tiles_material,
                    &args.render_target_meta,
                    &SHADOW_MAP_ATLAS_CLEAR_TILE_LAYOUT,
                )?;

            //
            // Draw quads over tiles that need to be cleared
            //
            let command_buffer = &args.command_buffer;
            command_buffer
                .cmd_bind_pipeline(&*pipeline.get_raw().pipeline)
                .unwrap();
            command_buffer.cmd_bind_vertex_buffers(
                0,
                &[RafxVertexBufferBinding {
                    buffer: &*vertex_buffer.get_raw().buffer,
                    byte_offset: 0,
                }],
            )?;
            command_buffer.cmd_draw(vertex_count as u32, 0).unwrap();
        }

        //
        // Draw the shadow maps
        //
        let mut write_context =
            RenderJobCommandBufferContext::from_graph_visit_render_pass_args(&args);

        for render_view in &render_views_to_draw {
            args.graph_context
                .prepared_render_data()
                .write_view_phase::<ShadowMapRenderPhase>(render_view, &mut write_context)?;
        }

        Ok(())
    });

    //context
    //    .render_resources
    //    .fetch_mut::<DebugPipRenderResource>()
    //    .add_render_graph_image(shadow_atlas_usage);

    ShadowMapPassOutput {
        shadow_atlas_image: shadow_atlas_usage,
    }
}
