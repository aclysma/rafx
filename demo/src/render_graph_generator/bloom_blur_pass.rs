use crate::phases::PostProcessRenderPhase;
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::graph::*;
use rafx::nodes::RenderPhase;

use super::RenderGraphContext;
use super::EMPTY_VERTEX_LAYOUT;
use rafx::api::RafxSampleCount;

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
            samples: Some(RafxSampleCount::SampleCount1),
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
        Default::default(),
    );
    context.graph.set_image_name(blur_dst, "blur_dst");

    let sample_image =
        context
            .graph
            .sample_image(node, blur_src, Default::default(), Default::default());
    context.graph.set_image_name(blur_src, "blur_src");

    let bloom_blur_material_pass = bloom_blur_material_pass.clone();
    context.graph.set_renderpass_callback(node, move |args| {
        // Get the color image from before
        let sample_image = args.graph_context.image_view(sample_image);

        // Get the pipeline
        let pipeline = args
            .graph_context
            .resource_context()
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                PostProcessRenderPhase::render_phase_index(),
                &bloom_blur_material_pass,
                &args.render_target_meta,
                &EMPTY_VERTEX_LAYOUT,
            )?;

        let descriptor_set_layouts = &pipeline.get_raw().descriptor_set_layouts;

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
        let command_buffer = &args.command_buffer;
        command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;
        bloom_blur_material_dyn_set.bind(command_buffer)?;
        command_buffer.cmd_draw(3, 0)?;

        Ok(())
    });

    blur_dst
}
