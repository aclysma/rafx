use super::BasicPipelineRenderOptions;
use crate::phases::PostProcessRenderPhase;
use crate::pipelines::basic::graph_generator::luma_pass::LumaAverageHistogramPass;
use crate::pipelines::basic::BasicPipelineOutputColorSpace;
use rafx::api::RafxSwapchainColorSpace;
use rafx::framework::{DescriptorSetBindings, MaterialPassResource, ResourceArc};
use rafx::graph::*;
use rafx::render_features::RenderPhase;
use rafx::renderer::SwapchainRenderResource;

use super::BloomExtractPass;
use super::RenderGraphContext;
use super::EMPTY_VERTEX_LAYOUT;
use crate::shaders::post_basic::bloom_combine_frag;

pub(super) struct BloomCombinePass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) color: RenderGraphImageUsageId,
}

pub(super) fn bloom_combine_pass(
    context: &mut RenderGraphContext,
    bloom_combine_material_pass: ResourceArc<MaterialPassResource>,
    bloom_extract_pass: &BloomExtractPass,
    blurred_color: RenderGraphImageUsageId,
    luma_average_histogram_pass: &LumaAverageHistogramPass,
    swapchain_render_resource: &SwapchainRenderResource,
) -> BloomCombinePass {
    let render_options = context
        .extract_resources
        .fetch::<BasicPipelineRenderOptions>()
        .clone();
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

    let histogram_result = context.graph.read_storage_buffer(
        node,
        luma_average_histogram_pass.histogram_result,
        Default::default(),
    );

    let swapchain_color_space = swapchain_render_resource
        .surface_info()
        .unwrap()
        .swapchain_surface_info
        .color_space;
    let max_color_component_value = swapchain_render_resource.max_color_component_value;

    context.graph.set_renderpass_callback(node, move |args| {
        // Get the color image from before
        let sdr_image = args.graph_context.image_view(sdr_image).unwrap();
        let hdr_image = args.graph_context.image_view(hdr_image).unwrap();
        let histogram_result = args.graph_context.buffer(histogram_result).unwrap();

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

        let output_color_space = match swapchain_color_space {
            RafxSwapchainColorSpace::Srgb => BasicPipelineOutputColorSpace::Srgb,
            RafxSwapchainColorSpace::SrgbExtended => BasicPipelineOutputColorSpace::Srgb,
            RafxSwapchainColorSpace::DisplayP3Extended => BasicPipelineOutputColorSpace::P3,
        };

        let descriptor_set_layouts = &pipeline.get_raw().descriptor_set_layouts;
        let mut bloom_combine_material_dyn_set = descriptor_set_allocator
            .create_dyn_descriptor_set(
                &descriptor_set_layouts[bloom_combine_frag::IN_COLOR_DESCRIPTOR_SET_INDEX],
                bloom_combine_frag::DescriptorSet0Args {
                    in_color: &sdr_image,
                    in_blur: &hdr_image,
                    config: &bloom_combine_frag::ConfigStd140 {
                        tonemapper_type: render_options.tonemapper_type as i32,
                        output_color_space: output_color_space as i32,
                        max_color_component_value,
                        ..Default::default()
                    },
                },
            )?;

        bloom_combine_material_dyn_set.0.set_buffer(
            bloom_combine_frag::HISTOGRAM_RESULT_DESCRIPTOR_BINDING_INDEX as u32,
            &histogram_result,
        );

        bloom_combine_material_dyn_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;

        command_buffer
            .cmd_bind_pipeline(&*pipeline.get_raw().pipeline)
            .unwrap();
        bloom_combine_material_dyn_set
            .0
            .bind(command_buffer)
            .unwrap();
        command_buffer.cmd_draw(3, 0).unwrap();

        Ok(())
    });

    BloomCombinePass { node, color }
}
