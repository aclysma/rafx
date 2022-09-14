use crate::pipelines::modern::graph_generator::ModernPipelineContext;
use crate::pipelines::modern::TemporalAAOptions;
use crate::shaders::post_adv::taa_frag;
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::graph::{
    RenderGraphExternalImageId, RenderGraphImageConstraint, RenderGraphImageUsageId,
    RenderGraphQueue,
};
use rafx::renderer::MainViewRenderResource;

pub(super) struct TaaPass {
    pub(super) color_rt: RenderGraphImageUsageId,
}

pub(super) fn taa_pass(
    context: &mut ModernPipelineContext,
    taa_options: &TemporalAAOptions,
    taa_material_pass: ResourceArc<MaterialPassResource>,
    color_rt: RenderGraphImageUsageId,
    depth_rt: RenderGraphImageUsageId,
    velocity_rt: RenderGraphImageUsageId,
    taa_history_rt_external_image_id: RenderGraphExternalImageId,
    taa_history_rt_has_data: bool,
) -> TaaPass {
    let node = context
        .graph
        .add_renderpass_node("TaaPass", RenderGraphQueue::DefaultGraphics);

    let velocity_rt = context.graph.sample_image(
        node,
        velocity_rt,
        RenderGraphImageConstraint::default(),
        Default::default(),
    );

    let current_rt = context.graph.sample_image(
        node,
        color_rt,
        RenderGraphImageConstraint::default(),
        Default::default(),
    );

    let depth_rt = context.graph.sample_image(
        node,
        depth_rt,
        RenderGraphImageConstraint::default(),
        Default::default(),
    );

    let taa_history_rt = context
        .graph
        .read_external_image(taa_history_rt_external_image_id);
    let taa_history_rt = context.graph.sample_image(
        node,
        taa_history_rt,
        RenderGraphImageConstraint::default(),
        Default::default(),
    );

    let color_rt = context.graph.create_color_attachment(
        node,
        0,
        None,
        RenderGraphImageConstraint {
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
        Default::default(),
    );

    let taa_material_pass = taa_material_pass.clone();
    let taa_options = taa_options.clone();
    let jitter_amount = context.graph_config.jitter_amount;
    context.graph.set_renderpass_callback(node, move |args| {
        let history_tex = args.graph_context.image_view(taa_history_rt).unwrap();
        let current_tex = args.graph_context.image_view(current_rt).unwrap();
        let velocity_tex = args.graph_context.image_view(velocity_rt).unwrap();
        let depth_tex = args.graph_context.image_view(depth_rt).unwrap();
        let pipeline = args
            .graph_context
            .resource_context()
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                None,
                &taa_material_pass,
                &args.render_target_meta,
                &super::EMPTY_VERTEX_LAYOUT,
            )?;
        let descriptor_set_layouts = &pipeline.get_raw().descriptor_set_layouts;
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();

        let main_view_resource = args
            .graph_context
            .render_resources()
            .fetch::<MainViewRenderResource>();
        let main_view = main_view_resource.main_view.clone().unwrap();
        let current_view_proj_inv = main_view.view_proj().inverse();
        let previous_view_proj =
            if let Some(previous_main_view_info) = &main_view_resource.previous_main_view_info {
                previous_main_view_info.projection_matrix * previous_main_view_info.view_matrix
            } else {
                main_view.view_proj()
            };

        let descriptor_set = descriptor_set_allocator.create_descriptor_set(
            &descriptor_set_layouts[taa_frag::CONFIG_DESCRIPTOR_SET_INDEX],
            taa_frag::DescriptorSet0Args {
                history_tex: &history_tex,
                current_tex: &current_tex,
                velocity_tex: &velocity_tex,
                depth_tex: &depth_tex,
                config: &taa_frag::ConfigUniform {
                    current_view_proj_inv: current_view_proj_inv.to_cols_array_2d(),
                    previous_view_proj: previous_view_proj.to_cols_array_2d(),
                    jitter_amount: jitter_amount.into(),
                    has_history_data: taa_history_rt_has_data as u32,
                    enable_side_by_side_debug_view: taa_options.enable_side_by_side_debug_view
                        as u32,
                    history_weight: taa_options.history_weight,
                    history_weight_velocity_adjust_multiplier: taa_options
                        .history_weight_velocity_adjust_multiplier,
                    history_weight_velocity_adjust_max: taa_options
                        .history_weight_velocity_adjust_max,
                    viewport_width: main_view.extents_width(),
                    viewport_height: main_view.extents_height(),
                    ..Default::default()
                },
            },
        )?;

        // Explicit flush since we're going to use the descriptors immediately
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;
        command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;
        command_buffer.cmd_draw(3, 0)?;

        Ok(())
    });

    let taa_history_rt = context.graph.copy_image_to_image(
        "copy color to history",
        RenderGraphQueue::DefaultGraphics,
        color_rt,
        Some(taa_history_rt),
        None,
    );

    context
        .graph
        .write_external_image(taa_history_rt_external_image_id, taa_history_rt);

    //let color_rt = context.graph.blit_image_to_image(
    //    "debug draw history",
    //    RenderGraphQueue::DefaultGraphics,
    //    taa_history_rt,
    //    glam::Vec2::ZERO,
    //    glam::Vec2::ONE,
    //    color_rt,
    //    glam::Vec2::ZERO,
    //    glam::Vec2::splat(0.25),
    //);

    //let color_rt = context.graph.blit_image_to_image(
    //    "debug draw velocity",
    //    RenderGraphQueue::DefaultGraphics,
    //    velocity_rt,
    //    glam::Vec2::ZERO,
    //    glam::Vec2::ONE,
    //    color_rt,
    //    glam::Vec2::new(0.0, 0.25),
    //    glam::Vec2::new(0.25, 0.50),
    //);

    TaaPass { color_rt }
}
