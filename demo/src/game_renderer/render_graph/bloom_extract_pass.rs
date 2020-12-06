use ash::version::DeviceV1_0;
use ash::vk;
use rafx::graph::*;
use rafx::resources::{MaterialPassResource, ResourceArc};

use super::OpaquePass;
use super::RenderGraphContext;
use super::EMPTY_VERTEX_LAYOUT;

pub(super) struct BloomExtractPass {
    pub(super) node: RenderGraphNodeId,
    pub(super) sdr_image: RenderGraphImageUsageId,
    pub(super) hdr_image: RenderGraphImageUsageId,
}

pub(super) fn bloom_extract_pass(
    context: &mut RenderGraphContext,
    bloom_extract_material_pass: ResourceArc<MaterialPassResource>,
    opaque_pass: &OpaquePass,
) -> BloomExtractPass {
    let node = context
        .graph
        .add_node("BloomExtract", RenderGraphQueue::DefaultGraphics);

    let sdr_image = context.graph.create_color_attachment(
        node,
        0,
        Default::default(),
        RenderGraphImageConstraint {
            samples: Some(vk::SampleCountFlags::TYPE_1),
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
    );
    context.graph.set_image_name(sdr_image, "sdr");
    let hdr_image = context.graph.create_color_attachment(
        node,
        1,
        Some(vk::ClearColorValue::default()),
        RenderGraphImageConstraint {
            samples: Some(vk::SampleCountFlags::TYPE_1),
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
    );
    context.graph.set_image_name(hdr_image, "hdr");

    let sample_image = context.graph.sample_image(
        node,
        opaque_pass.color,
        RenderGraphImageConstraint {
            samples: Some(vk::SampleCountFlags::TYPE_1),
            ..Default::default()
        },
        Default::default(),
        Default::default(),
    );

    context
        .graph_callbacks
        .set_renderpass_callback(node, move |args, _user_context| {
            // Get the color image from before
            let sample_image = args.graph_context.image_view(sample_image);

            // Get the pipeline
            let pipeline = args
                .graph_context
                .resource_context()
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    &bloom_extract_material_pass,
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
            let bloom_extract_material_dyn_set = descriptor_set_allocator.create_descriptor_set(
                &descriptor_set_layouts[shaders::bloom_extract_frag::TEX_DESCRIPTOR_SET_INDEX],
                shaders::bloom_extract_frag::DescriptorSet0Args {
                    tex: sample_image.as_ref().unwrap(),
                },
            )?;

            // Explicit flush since we're going to use the descriptors immediately
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
                    shaders::bloom_extract_frag::TEX_DESCRIPTOR_SET_INDEX as u32,
                    &[bloom_extract_material_dyn_set.get()],
                    &[],
                );

                device.cmd_draw(command_buffer, 3, 1, 0, 0);
            }

            Ok(())
        });

    BloomExtractPass {
        node,
        sdr_image,
        hdr_image,
    }
}
