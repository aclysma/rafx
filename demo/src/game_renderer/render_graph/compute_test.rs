use rafx::graph::*;

use super::RenderGraphContext;
use ash::version::DeviceV1_0;
use ash::vk;
use rafx::resources::{ComputePipelineResource, ResourceArc};

pub(super) struct ComputeTestPass {
    pub(super) node: RenderGraphNodeId,
    pub(super) position_buffer: RenderGraphBufferUsageId,
    pub(super) velocity_buffer: RenderGraphBufferUsageId,
}

pub(super) fn compute_test_pass(
    context: &mut RenderGraphContext,
    test_compute_pipeline: &ResourceArc<ComputePipelineResource>,
) -> ComputeTestPass {
    let node = context
        .graph
        .add_node("compute_test", RenderGraphQueue::DefaultGraphics);

    let position_buffer_size = std::mem::size_of::<shaders::compute_test_comp::PositionsBuffer>();
    let velocity_buffer_size = std::mem::size_of::<shaders::compute_test_comp::VelocityBuffer>();

    let position_buffer = context.graph.create_storage_buffer(
        node,
        RenderGraphBufferConstraint {
            size: Some(position_buffer_size as u64),
            ..Default::default()
        },
    );

    let velocity_buffer = context.graph.create_storage_buffer(
        node,
        RenderGraphBufferConstraint {
            size: Some(velocity_buffer_size as u64),
            ..Default::default()
        },
    );

    let test_compute_pipeline = test_compute_pipeline.clone();

    context
        .graph_callbacks
        .set_compute_callback(node, move |args, _user_context| {
            let mut descriptor_set_allocator = args
                .graph_context
                .resource_context()
                .create_descriptor_set_allocator();
            let mut descriptor_set = descriptor_set_allocator
                .create_dyn_descriptor_set_uninitialized(
                    &test_compute_pipeline
                        .get_raw()
                        .pipeline_layout
                        .get_raw()
                        .descriptor_sets[0],
                )?;

            let positions = args.graph_context.buffer(position_buffer).unwrap();
            let velocities = args.graph_context.buffer(velocity_buffer).unwrap();

            descriptor_set.set_buffer(0, &positions);
            descriptor_set.set_buffer(1, &velocities);
            descriptor_set.flush(&mut descriptor_set_allocator)?;
            descriptor_set_allocator.flush_changes()?;

            // Draw calls
            let command_buffer = args.command_buffer;
            let device = args.graph_context.device_context().device();

            unsafe {
                device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    test_compute_pipeline.get_raw().pipeline,
                );

                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    test_compute_pipeline
                        .get_raw()
                        .pipeline_layout
                        .get_raw()
                        .pipeline_layout,
                    0,
                    &[descriptor_set.descriptor_set().get()],
                    &[],
                );

                device.cmd_dispatch(command_buffer, 100, 1, 1);
            }
            Ok(())
        });

    ComputeTestPass {
        node,
        position_buffer,
        velocity_buffer,
    }
}
