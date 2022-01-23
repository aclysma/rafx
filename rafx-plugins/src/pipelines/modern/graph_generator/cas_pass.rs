use rafx::framework::{ComputePipelineResource, DescriptorSetBindings, ResourceArc};
use rafx::graph::*;

use super::ModernPipelineContext;
use crate::shaders::cas::cas32_comp;
use rafx::api::{RafxFormat, RafxSampleCount};

pub(super) struct CasPass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) color_rt: RenderGraphImageUsageId,
}

pub(super) fn cas_pass(
    context: &mut ModernPipelineContext,
    cas_pipeline: &ResourceArc<ComputePipelineResource>,
    color_rt: RenderGraphImageUsageId,
    swapchain_surface_info: &SwapchainSurfaceInfo,
) -> CasPass {
    let node = context
        .graph
        .add_node("Cas", RenderGraphQueue::DefaultGraphics);

    let src_rt = context.graph.read_storage_image(
        node,
        color_rt,
        RenderGraphImageConstraint {
            samples: Some(RafxSampleCount::SampleCount1),
            format: Some(RafxFormat::R16G16B16A16_SFLOAT),
            ..Default::default()
        },
        Default::default(),
    );

    let dst_rt = context.graph.create_storage_image(
        node,
        RenderGraphImageConstraint {
            samples: Some(RafxSampleCount::SampleCount1),
            format: Some(RafxFormat::R16G16B16A16_SFLOAT),
            //format: Some(context.graph_config.color_format),
            ..Default::default()
        },
        Default::default(),
    );

    let cas_pipeline = cas_pipeline.clone();

    let swapchain_extents = swapchain_surface_info.extents;
    let sharpening_amount = context.graph_config.sharpening_amount;

    context.graph.set_callback(node, move |args| {
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();
        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &cas_pipeline.get_raw().descriptor_set_layouts[0],
        )?;

        let src_rt = args.graph_context.image_view(src_rt).unwrap();
        let dst_rt = args.graph_context.image_view(dst_rt).unwrap();

        let input_width = swapchain_extents.width;
        let input_height = swapchain_extents.height;

        descriptor_set.set_buffer_data(
            cas32_comp::CONFIG_DESCRIPTOR_BINDING_INDEX as u32,
            &cas32_comp::ConfigUniform {
                image_width: input_width,
                image_height: input_height,
                sharpen_amount: sharpening_amount,
                _padding0: Default::default(),
            },
        );
        descriptor_set.set_image(cas32_comp::IMG_SRC_DESCRIPTOR_BINDING_INDEX as u32, &src_rt);
        descriptor_set.set_image(cas32_comp::IMG_DST_DESCRIPTOR_BINDING_INDEX as u32, &dst_rt);
        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;

        command_buffer.cmd_bind_pipeline(&*cas_pipeline.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;
        command_buffer.cmd_dispatch((input_width + 15) / 16, (input_height + 15) / 16, 1)?;

        Ok(())
    });

    CasPass {
        node,
        color_rt: dst_rt,
    }
}
