use crate::phases::PostProcessRenderPhase;
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::graph::*;
use rafx::nodes::RenderPhase;

use super::OpaquePass;
use super::RenderGraphContext;
use super::EMPTY_VERTEX_LAYOUT;
use rafx::api::{RafxColorClearValue, RafxSampleCount};

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
            samples: Some(RafxSampleCount::SampleCount1),
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
        Default::default(),
    );
    context.graph.set_image_name(sdr_image, "sdr");
    let hdr_image = context.graph.create_color_attachment(
        node,
        1,
        Some(RafxColorClearValue::default()),
        RenderGraphImageConstraint {
            samples: Some(RafxSampleCount::SampleCount1),
            format: Some(context.graph_config.color_format),
            ..Default::default()
        },
        Default::default(),
    );
    context.graph.set_image_name(hdr_image, "hdr");

    let sample_image = context.graph.sample_image(
        node,
        opaque_pass.color,
        RenderGraphImageConstraint {
            samples: Some(RafxSampleCount::SampleCount1),
            ..Default::default()
        },
        Default::default(),
    );

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
                &bloom_extract_material_pass,
                &args.render_target_meta,
                &EMPTY_VERTEX_LAYOUT,
            )?;

        // Set up a descriptor set pointing at the image so we can sample from it
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();

        let descriptor_set_layouts = &pipeline.get_raw().descriptor_set_layouts;
        let bloom_extract_material_dyn_set = descriptor_set_allocator.create_descriptor_set(
            &descriptor_set_layouts[shaders::bloom_extract_frag::TEX_DESCRIPTOR_SET_INDEX],
            shaders::bloom_extract_frag::DescriptorSet0Args {
                tex: sample_image.as_ref().unwrap(),
            },
        )?;

        // Explicit flush since we're going to use the descriptors immediately
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;
        command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;
        bloom_extract_material_dyn_set.bind(command_buffer)?;
        command_buffer.cmd_draw(3, 0)?;

        Ok(())
    });

    BloomExtractPass {
        node,
        sdr_image,
        hdr_image,
    }
}
