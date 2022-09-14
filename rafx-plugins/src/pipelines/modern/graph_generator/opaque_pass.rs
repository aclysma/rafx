use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase, WireframeRenderPhase};
use rafx::graph::*;

use super::ModernPipelineContext;
use crate::assets::mesh_adv::MeshAdvShaderPassIndices;
use crate::features::mesh_adv::{MeshAdvRenderPipelineState, MeshAdvStaticResources};
use crate::pipelines::modern::graph_generator::light_binning::LightBuildListsPass;
use crate::pipelines::modern::graph_generator::shadow_map_pass::ShadowMapPassOutput;
use crate::shaders::mesh_adv::mesh_adv_textured_frag;
use rafx::api::RafxColorClearValue;
use rafx::render_features::RenderJobCommandBufferContext;
use rafx::renderer::InvalidResources;

pub(super) struct OpaquePass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) color: RenderGraphImageUsageId,
    #[allow(dead_code)]
    pub(super) shadow_map_atlas: RenderGraphImageUsageId,
}

pub(super) fn opaque_pass(
    context: &mut ModernPipelineContext,
    depth_prepass: RenderGraphImageUsageId,
    shadow_map_pass_output: &ShadowMapPassOutput,
    light_build_lists_pass: &LightBuildListsPass,
    ssao_rt: Option<RenderGraphImageUsageId>,
) -> OpaquePass {
    let node = context
        .graph
        .add_renderpass_node("Opaque", RenderGraphQueue::DefaultGraphics);

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

    if context.graph_config.show_surfaces {
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
    }

    let ssao_rt = ssao_rt.map(|x| {
        context.graph.sample_image(
            node,
            x,
            RenderGraphImageConstraint::default(),
            RenderGraphImageViewOptions::default(),
        )
    });

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

    let default_pbr_material = context
        .render_resources
        .fetch::<MeshAdvStaticResources>()
        .default_pbr_material
        .clone();
    let default_pbr_material = context
        .asset_manager
        .committed_asset(&default_pbr_material)
        .unwrap()
        .clone();

    context.graph.set_renderpass_callback(node, move |args| {
        let mut write_context =
            RenderJobCommandBufferContext::from_graph_visit_render_pass_args(&args);

        let invalid_image = args
            .graph_context
            .render_resources()
            .fetch::<InvalidResources>()
            .invalid_image_color
            .clone();

        let ssao_rt = ssao_rt.map(|x| args.graph_context.image_view(x).unwrap());
        let ssao_rt = ssao_rt.unwrap_or(invalid_image);

        let default_pbr_material_pass_indices =
            MeshAdvShaderPassIndices::new(&default_pbr_material);
        let default_pass = default_pbr_material
            .get_material_pass_by_index(default_pbr_material_pass_indices.opaque as usize)
            .unwrap();

        let descriptor_set_layouts = &default_pass.get_raw().descriptor_set_layouts;
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();
        let descriptor_set = descriptor_set_allocator.create_descriptor_set(
            &descriptor_set_layouts[mesh_adv_textured_frag::SSAO_TEXTURE_DESCRIPTOR_SET_INDEX],
            mesh_adv_textured_frag::DescriptorSet1Args {
                ssao_texture: &ssao_rt,
            },
        )?;
        descriptor_set_allocator.flush_changes()?;

        args.graph_context
            .render_resources()
            .fetch_mut::<MeshAdvRenderPipelineState>()
            .ssao_descriptor_set = Some(descriptor_set);

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

        args.graph_context
            .render_resources()
            .fetch_mut::<MeshAdvRenderPipelineState>()
            .ssao_descriptor_set = None;

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
