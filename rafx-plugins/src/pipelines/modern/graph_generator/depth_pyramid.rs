use rafx::framework::{ComputePipelineResource, DescriptorSetBindings, ResourceArc};
use rafx::graph::*;

use super::ModernPipelineContext;
use crate::shaders::depth::depth_pyramid_comp;
use rafx::api::{RafxExtents3D, RafxFormat, RafxSampleCount};

pub const MAX_DEPTH_PYRAMID_MIP_LAYERS: u32 = 16;

pub(super) struct DepthPyramidPass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) depth_pyramid_mips: Vec<RenderGraphImageUsageId>,
}

fn depth_pyramid_mip_pass(
    context: &mut ModernPipelineContext,
    depth_pyramid_pipeline: &ResourceArc<ComputePipelineResource>,
    src_depth_rt: RenderGraphImageUsageId,
    dst_depth_rt: RenderGraphImageUsageId,
    input_width: u32,
    input_height: u32,
    node: RenderGraphNodeId,
) {
    let depth_pyramid_pipeline = depth_pyramid_pipeline.clone();

    context.graph.set_callback(node, move |args| {
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();
        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &depth_pyramid_pipeline.get_raw().descriptor_set_layouts[0],
        )?;

        let src_depth_rt = args.graph_context.image_view(src_depth_rt).unwrap();
        let dst_depth_rt = args.graph_context.image_view(dst_depth_rt).unwrap();

        descriptor_set.set_buffer_data(
            depth_pyramid_comp::CONFIG_DESCRIPTOR_BINDING_INDEX as u32,
            &depth_pyramid_comp::DepthPyramidConfigUniform {
                input_width,
                input_height,
                odd_width: ((input_width % 2) == 1) as u32,
                odd_height: ((input_height % 2) == 1) as u32,
            },
        );
        descriptor_set.set_image(
            depth_pyramid_comp::SRC_DEPTH_TEX_DESCRIPTOR_BINDING_INDEX as u32,
            &src_depth_rt,
        );
        descriptor_set.set_image(
            depth_pyramid_comp::DST_DEPTH_TEX_DESCRIPTOR_BINDING_INDEX as u32,
            &dst_depth_rt,
        );
        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;

        command_buffer.cmd_bind_pipeline(&*depth_pyramid_pipeline.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;
        let group_count_x = 1.max(input_width / 2);
        let group_count_y = 1.max(input_height / 2);
        command_buffer.cmd_dispatch((group_count_x + 15) / 16, (group_count_y + 15) / 16, 1)?;

        Ok(())
    });
}

pub(super) fn depth_pyramid_pass(
    context: &mut ModernPipelineContext,
    depth_pyramid_pipeline: &ResourceArc<ComputePipelineResource>,
    src_depth_rt: RenderGraphImageUsageId,
    swapchain_surface_info: &SwapchainSurfaceInfo,
) -> DepthPyramidPass {
    let mip_levels = rafx::api::extra::mipmaps::mip_level_max_count_for_image_size(
        swapchain_surface_info.extents.width,
        swapchain_surface_info.extents.height,
    )
    .min(MAX_DEPTH_PYRAMID_MIP_LAYERS);

    let swapchain_extents = swapchain_surface_info.extents;

    let mut previous_node = None;
    let mut previous_dst_depth_rt = None;

    let mut depth_images = Vec::with_capacity(mip_levels as usize);
    depth_images.push(src_depth_rt);

    for dst_mip_level in 1..mip_levels {
        let node = context
            .graph
            .add_callback_node("DepthPyramid", RenderGraphQueue::DefaultGraphics);

        let input_width = 1.max(swapchain_extents.width >> (dst_mip_level - 1));
        let input_height = 1.max(swapchain_extents.height >> (dst_mip_level - 1));
        let output_width = 1.max(swapchain_extents.width >> dst_mip_level);
        let output_height = 1.max(swapchain_extents.height >> dst_mip_level);

        //println!(
        //    "{}x{} -> {}x{}",
        //    input_width, input_height, output_width, output_height
        //);

        let src_depth_rt = if dst_mip_level == 1 {
            context.graph.sample_image(
                node,
                src_depth_rt,
                RenderGraphImageConstraint {
                    samples: Some(context.graph_config.samples),
                    format: Some(RafxFormat::D32_SFLOAT),
                    ..Default::default()
                },
                Default::default(),
            )
        } else {
            context.graph.sample_image(
                node,
                previous_dst_depth_rt.unwrap(),
                RenderGraphImageConstraint {
                    samples: Some(RafxSampleCount::SampleCount1),
                    format: Some(RafxFormat::R32_SFLOAT),
                    ..Default::default()
                },
                Default::default(),
            )
        };

        let dst_depth_rt = context.graph.create_storage_image(
            node,
            RenderGraphImageConstraint {
                samples: Some(RafxSampleCount::SampleCount1),
                format: Some(RafxFormat::R32_SFLOAT),
                extents: Some(RenderGraphImageExtents::Custom(RafxExtents3D {
                    width: output_width,
                    height: output_height,
                    depth: 1,
                })),
                ..Default::default()
            },
            Default::default(),
        );

        depth_pyramid_mip_pass(
            context,
            depth_pyramid_pipeline,
            src_depth_rt,
            dst_depth_rt,
            input_width,
            input_height,
            node,
        );
        previous_node = Some(node);
        previous_dst_depth_rt = Some(dst_depth_rt);
        depth_images.push(dst_depth_rt);
    }

    DepthPyramidPass {
        node: previous_node.unwrap(),
        depth_pyramid_mips: depth_images,
    }
}
