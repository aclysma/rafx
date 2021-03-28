use crate::{phases::PostProcessRenderPhase, RenderOptions};
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::graph::*;
use rafx::nodes::RenderPhase;

use super::BloomExtractPass;
use super::RenderGraphContext;
use super::EMPTY_VERTEX_LAYOUT;

pub(super) struct BloomCombinePass {
    pub(super) node: RenderGraphNodeId,
    pub(super) color: RenderGraphImageUsageId,
}

pub(super) fn bloom_combine_pass(
    context: &mut RenderGraphContext,
    bloom_combine_material_pass: ResourceArc<MaterialPassResource>,
    bloom_extract_pass: &BloomExtractPass,
    blurred_color: RenderGraphImageUsageId,
) -> BloomCombinePass {
    let render_options = context.extract_resources.fetch::<RenderOptions>().clone();
    let node = context
        .graph
        .add_node("BloomCombine", RenderGraphQueue::DefaultGraphics);

    let color = context.graph.create_color_attachment(
        node,
        0,
        Default::default(),
        RenderGraphImageConstraint {
            format: Some(context.graph_config.swapchain_format),
            ..Default::default()
        },
        Default::default(),
    );
    context.graph.set_image_name(color, "color");

    let sdr_image = context.graph.sample_image(
        node,
        bloom_extract_pass.sdr_image,
        Default::default(),
        Default::default(),
    );
    context.graph.set_image_name(sdr_image, "sdr");

    let hdr_image =
        context
            .graph
            .sample_image(node, blurred_color, Default::default(), Default::default());
    context.graph.set_image_name(hdr_image, "hdr");

    context.graph.set_renderpass_callback(node, move |args| {
        // Get the color image from before
        let sdr_image = args.graph_context.image_view(sdr_image).unwrap();
        let hdr_image = args.graph_context.image_view(hdr_image).unwrap();

        // Get the pipeline
        let pipeline = args
            .graph_context
            .resource_context()
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                PostProcessRenderPhase::render_phase_index(),
                &bloom_combine_material_pass,
                &args.render_target_meta,
                &EMPTY_VERTEX_LAYOUT,
            )?;

        // Set up a descriptor set pointing at the image so we can sample from it
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();

        let descriptor_set_layouts = &pipeline.get_raw().descriptor_set_layouts;
        let bloom_combine_material_dyn_set = descriptor_set_allocator.create_descriptor_set(
            &descriptor_set_layouts[shaders::bloom_combine_frag::IN_COLOR_DESCRIPTOR_SET_INDEX],
            shaders::bloom_combine_frag::DescriptorSet0Args {
                in_color: &sdr_image,
                in_blur: &hdr_image,
                config: &shaders::bloom_combine_frag::ConfigStd140 {
                    tonemapper_type: render_options.tonemapper_type as i32,
                    ..Default::default()
                },
            },
        )?;

        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;
        command_buffer
            .cmd_bind_pipeline(&*pipeline.get_raw().pipeline)
            .unwrap();
        bloom_combine_material_dyn_set.bind(command_buffer).unwrap();
        command_buffer.cmd_draw(3, 0).unwrap();

        Ok(())
    });

    BloomCombinePass { node, color }
}
