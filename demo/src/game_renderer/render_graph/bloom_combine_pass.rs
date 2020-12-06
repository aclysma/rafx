use ash::version::DeviceV1_0;
use ash::vk;
use rafx::graph::*;
use rafx::resources::{MaterialPassResource, ResourceArc};

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
    );
    context.graph.set_image_name(color, "color");

    let sdr_image = context.graph.sample_image(
        node,
        bloom_extract_pass.sdr_image,
        Default::default(),
        Default::default(),
        Default::default(),
    );
    context.graph.set_image_name(sdr_image, "sdr");

    let hdr_image = context.graph.sample_image(
        node,
        blurred_color,
        Default::default(),
        Default::default(),
        Default::default(),
    );
    context.graph.set_image_name(hdr_image, "hdr");

    context
        .graph_callbacks
        .set_renderpass_callback(node, move |args, _user_context| {
            // Get the color image from before
            let sdr_image = args.graph_context.image_view(sdr_image).unwrap();
            let hdr_image = args.graph_context.image_view(hdr_image).unwrap();

            // Get the pipeline
            let pipeline = args
                .graph_context
                .resource_context()
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    &bloom_combine_material_pass,
                    args.renderpass_resource,
                    &args
                        .framebuffer_resource
                        .get_raw()
                        .framebuffer_key
                        .framebuffer_meta,
                    &EMPTY_VERTEX_LAYOUT,
                )?;

            // Set up a descriptor set pointing at the image so we can sample from it
            let mut descriptor_set_allocator = args
                .graph_context
                .resource_context()
                .create_descriptor_set_allocator();

            let descriptor_set_layouts =
                &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets;
            let bloom_combine_material_dyn_set = descriptor_set_allocator.create_descriptor_set(
                &descriptor_set_layouts[shaders::bloom_combine_frag::IN_COLOR_DESCRIPTOR_SET_INDEX],
                shaders::bloom_combine_frag::DescriptorSet0Args {
                    in_color: &sdr_image,
                    in_blur: &hdr_image,
                },
            )?;

            descriptor_set_allocator.flush_changes()?;

            // Draw calls
            let command_buffer = args.command_buffer;
            let device = args.graph_context.device_context().device();

            unsafe {
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipelines[0],
                );

                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                    shaders::bloom_combine_frag::IN_COLOR_DESCRIPTOR_SET_INDEX as u32,
                    &[bloom_combine_material_dyn_set.get()],
                    &[],
                );

                device.cmd_draw(command_buffer, 3, 1, 0, 0);
            }

            Ok(())
        });

    BloomCombinePass { node, color }
}
