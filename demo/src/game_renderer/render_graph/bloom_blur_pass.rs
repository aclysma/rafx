use ash::version::DeviceV1_0;
use ash::vk;
use rafx::graph::*;
use rafx::resources::{MaterialPassResource, ResourceArc};

use super::RenderGraphContext;
use super::EMPTY_VERTEX_LAYOUT;

#[derive(PartialEq)]
pub(super) enum BlurDirection {
    Horizontal,
    Vertical,
}

pub(super) struct BloomBlurPass {
    pub(super) color: RenderGraphImageUsageId,
}

pub(super) fn bloom_blur_pass(
    context: &mut RenderGraphContext,
    bloom_blur_material_pass: ResourceArc<MaterialPassResource>,
    bloom_extract_pass: &super::BloomExtractPass,
) -> BloomBlurPass {
    let mut blur_src = bloom_extract_pass.hdr_image;

    for _ in 0..context.graph_config.blur_pass_count {
        blur_src = bloom_blur_internal_pass(
            context,
            &bloom_blur_material_pass,
            blur_src,
            BlurDirection::Vertical,
        );
        blur_src = bloom_blur_internal_pass(
            context,
            &bloom_blur_material_pass,
            blur_src,
            BlurDirection::Horizontal,
        );
    }

    return BloomBlurPass { color: blur_src };
}

fn bloom_blur_internal_pass(
    context: &mut RenderGraphContext,
    bloom_blur_material_pass: &ResourceArc<MaterialPassResource>,
    blur_src: RenderGraphImageUsageId,
    blur_direction: BlurDirection,
) -> RenderGraphImageUsageId {
    let node = context
        .graph
        .add_node("BloomBlur", RenderGraphQueue::DefaultGraphics);
    let blur_dst = context.graph.create_color_attachment(
        node,
        0,
        Some(Default::default()),
        RenderGraphImageConstraint {
            samples: Some(vk::SampleCountFlags::TYPE_1),
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
    );
    context.graph.set_image_name(blur_dst, "blur_dst");

    let sample_image = context.graph.sample_image(
        node,
        blur_src,
        Default::default(),
        Default::default(),
        Default::default(),
    );
    context.graph.set_image_name(blur_src, "blur_src");

    let bloom_blur_material_pass = bloom_blur_material_pass.clone();
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
                    &bloom_blur_material_pass,
                    args.renderpass_resource,
                    &args
                        .framebuffer_resource
                        .get_raw()
                        .framebuffer_key
                        .framebuffer_meta,
                    &EMPTY_VERTEX_LAYOUT,
                )?;

            let descriptor_set_layouts =
                &pipeline.get_raw().pipeline_layout.get_raw().descriptor_sets;

            // Set up a descriptor set pointing at the image so we can sample from it
            let mut descriptor_set_allocator = args
                .graph_context
                .resource_context()
                .create_descriptor_set_allocator();

            let horizontal = if blur_direction == BlurDirection::Horizontal {
                1
            } else {
                0
            };

            let bloom_blur_material_dyn_set = descriptor_set_allocator.create_descriptor_set(
                &descriptor_set_layouts[shaders::bloom_blur_frag::TEX_DESCRIPTOR_SET_INDEX],
                shaders::bloom_blur_frag::DescriptorSet0Args {
                    tex: sample_image.as_ref().unwrap(),
                    config: &shaders::bloom_blur_frag::ConfigUniform {
                        horizontal,
                        ..Default::default()
                    },
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
                    shaders::bloom_blur_frag::CONFIG_DESCRIPTOR_SET_INDEX as u32,
                    &[bloom_blur_material_dyn_set.get()],
                    &[],
                );

                device.cmd_draw(command_buffer, 3, 1, 0, 0);
            }

            Ok(())
        });

    blur_dst
}
