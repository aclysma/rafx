use rafx::framework::{ComputePipelineResource, DescriptorSetBindings, ResourceArc};
use rafx::graph::*;

use super::OpaquePass;
use super::RenderGraphContext;
use crate::pipelines::basic::BasicPipelineTonemapDebugData;
use crate::shaders;
use rafx::api::{RafxLoadOp, RafxSampleCount};

const LOG_LUMA_MIN: f32 = -10.0;
const LOG_LUMA_RANGE: f32 = 12.0;

pub(super) struct LumaBuildHistogramPass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) luma_histogram_data: RenderGraphBufferUsageId,
}

pub(super) fn luma_build_histogram_pass(
    context: &mut RenderGraphContext,
    luma_build_histogram: &ResourceArc<ComputePipelineResource>,
    opaque_pass: &OpaquePass,
    histogram_buffer: Option<RenderGraphExternalBufferId>,
    swapchain_surface_info: &SwapchainSurfaceInfo,
) -> LumaBuildHistogramPass {
    let node = context
        .graph
        .add_node("LumaBuildHistogram", RenderGraphQueue::DefaultGraphics);

    let luma_histogram_data = if let Some(histogram_buffer) = histogram_buffer {
        let usage = context.graph.read_external_buffer(histogram_buffer);

        context
            .graph
            .modify_storage_buffer(node, usage, Default::default(), RafxLoadOp::Clear)
    } else {
        context.graph.create_storage_buffer(
            node,
            RenderGraphBufferConstraint {
                size: Some(std::mem::size_of::<
                    shaders::luma_average_histogram_comp::HistogramDataBuffer,
                >() as u64),
                ..Default::default()
            },
            RafxLoadOp::Clear,
        )
    };

    let luma_sample_hdr_image = context.graph.sample_image(
        node,
        opaque_pass.color,
        RenderGraphImageConstraint {
            samples: Some(RafxSampleCount::SampleCount1),
            ..Default::default()
        },
        Default::default(),
    );

    let luma_build_histogram = luma_build_histogram.clone();

    let swapchain_extents = swapchain_surface_info.extents;

    context.graph.set_compute_callback(node, move |args| {
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();
        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &luma_build_histogram.get_raw().descriptor_set_layouts[0],
        )?;

        let luma_sample_hdr_image = args
            .graph_context
            .image_view(luma_sample_hdr_image)
            .unwrap();
        let histogram_data = args.graph_context.buffer(luma_histogram_data).unwrap();

        use crate::shaders::luma_build_histogram_comp;

        let input_width = swapchain_extents.width;
        let input_height = swapchain_extents.height;

        descriptor_set.set_buffer_data(
            luma_build_histogram_comp::CONFIG_DESCRIPTOR_BINDING_INDEX as u32,
            &luma_build_histogram_comp::BuildHistogramConfigUniform {
                input_height,
                input_width,
                min_log_luma: LOG_LUMA_MIN,
                one_over_log_luma_range: 1.0 / LOG_LUMA_RANGE,
            },
        );
        descriptor_set.set_image(
            luma_build_histogram_comp::TEX_DESCRIPTOR_BINDING_INDEX as u32,
            &luma_sample_hdr_image,
        );
        descriptor_set.set_buffer(
            luma_build_histogram_comp::HISTOGRAM_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            &histogram_data,
        );
        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;

        command_buffer.cmd_bind_pipeline(&*luma_build_histogram.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;
        command_buffer.cmd_dispatch((input_width + 15) / 16, (input_height + 15) / 16, 1)?;

        Ok(())
    });

    LumaBuildHistogramPass {
        node,
        luma_histogram_data,
    }
}

pub(super) struct LumaAverageHistogramPass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
    pub(super) histogram_result: RenderGraphBufferUsageId,
}

pub(super) fn luma_average_histogram_pass(
    context: &mut RenderGraphContext,
    luma_build_histogram_pass: &LumaBuildHistogramPass,
    luma_average_histogram: &ResourceArc<ComputePipelineResource>,
    histogram_result: RenderGraphExternalBufferId,
    tonemap_debug_data: Option<BasicPipelineTonemapDebugData>,
    swapchain_surface_info: &SwapchainSurfaceInfo,
    previous_update_dt: f32,
) -> LumaAverageHistogramPass {
    let node = context
        .graph
        .add_node("LumaAverageHistogram", RenderGraphQueue::DefaultGraphics);

    // We assume 16x16 workgroup size
    let luma_histogram_data = context.graph.read_storage_buffer(
        node,
        luma_build_histogram_pass.luma_histogram_data,
        RenderGraphBufferConstraint {
            size: Some(256 * std::mem::size_of::<u32>() as u64),
            ..Default::default()
        },
    );

    let histogram_result = context.graph.read_external_buffer(histogram_result);

    let histogram_result = context.graph.modify_storage_buffer(
        node,
        histogram_result,
        Default::default(),
        RafxLoadOp::Load,
    );

    let luma_average_histogram = luma_average_histogram.clone();

    let swapchain_extents = swapchain_surface_info.extents;

    context.graph.set_compute_callback(node, move |args| {
        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();
        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &luma_average_histogram.get_raw().descriptor_set_layouts[0],
        )?;

        let histogram_data = args.graph_context.buffer(luma_histogram_data).unwrap();
        let histogram_result = args.graph_context.buffer(histogram_result).unwrap();

        // Copy the result of previous frame's histogram to debug data resource
        unsafe {
            let histogram_data_ptr = histogram_data.get_raw().buffer.map_buffer()?;
            let histogram_data_ptr = &*(histogram_data_ptr
                as *mut shaders::luma_average_histogram_comp::HistogramDataBuffer);

            let histogram_result_ptr = histogram_result.get_raw().buffer.map_buffer()?;
            let histogram_result_ptr = &*(histogram_result_ptr
                as *mut shaders::luma_average_histogram_comp::HistogramResultBuffer);

            if let Some(tonemap_debug_data) = &tonemap_debug_data {
                let mut guard = tonemap_debug_data.inner.lock().unwrap();
                guard.histogram_sample_count = 0;
                guard.histogram_max_value = 0;
                for i in 0..256 {
                    let d = histogram_data_ptr.data[i];
                    guard.histogram[i] = d;
                    guard.histogram_sample_count += d;
                    guard.histogram_max_value = guard.histogram_max_value.max(d);
                }

                guard.result_average = histogram_result_ptr.average_luminosity_interpolated;
                guard.result_average_bin = histogram_result_ptr.average_bin_non_zero as f32;
                guard.result_min_bin = histogram_result_ptr.min_bin;
                guard.result_max_bin = histogram_result_ptr.max_bin;
                guard.result_low_bin = histogram_result_ptr.low_bin;
                guard.result_high_bin = histogram_result_ptr.high_bin;
                // println!("{:?}", *guard);
            }
            histogram_result.get_raw().buffer.unmap_buffer()?;
            histogram_data.get_raw().buffer.unmap_buffer()?;
        }

        use crate::shaders::luma_average_histogram_comp;

        let input_width = swapchain_extents.width;
        let input_height = swapchain_extents.height;

        descriptor_set.set_buffer_data(
            luma_average_histogram_comp::CONFIG_DESCRIPTOR_BINDING_INDEX as u32,
            &luma_average_histogram_comp::AverageHistogramConfigUniform {
                pixel_count: input_width * input_height,
                dt: previous_update_dt,
                min_log_luma: LOG_LUMA_MIN,
                log_luma_range: LOG_LUMA_RANGE,
                low_adjust_speed: 1.2,
                high_adjust_speed: 1.7,
                low_percentile: 0.1,
                high_percentile: 0.95,
            },
        );
        descriptor_set.set_buffer(
            luma_average_histogram_comp::HISTOGRAM_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            &histogram_data,
        );
        descriptor_set.set_buffer(
            luma_average_histogram_comp::HISTOGRAM_RESULT_DESCRIPTOR_BINDING_INDEX as u32,
            &histogram_result,
        );
        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;

        command_buffer.cmd_bind_pipeline(&*luma_average_histogram.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;
        command_buffer.cmd_dispatch(1, 1, 1)?;

        Ok(())
    });

    LumaAverageHistogramPass {
        node,
        histogram_result,
    }
}
