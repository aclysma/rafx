use rafx::framework::{ComputePipelineResource, DescriptorSetBindings, ResourceArc};
use rafx::graph::*;

use super::ModernPipelineContext;
use crate::features::mesh_adv::MeshAdvGpuOcclusionCullRenderResource;
use crate::pipelines::modern::graph_generator::depth_pyramid::{
    DepthPyramidPass, MAX_DEPTH_PYRAMID_MIP_LAYERS,
};
use crate::pipelines::modern::ModernPipelineMeshCullingDebugData;
use crate::shaders::mesh_adv::mesh_culling_comp;
use rafx::api::{RafxBarrierQueueTransition, RafxBufferBarrier, RafxLoadOp, RafxResourceState};
use rafx::renderer::InvalidResources;

pub(super) struct MeshCullingPass {
    #[allow(dead_code)]
    pub(super) node: RenderGraphNodeId,
}

pub(super) fn mesh_culling_pass(
    context: &mut ModernPipelineContext,
    mesh_culling_pipeline: &ResourceArc<ComputePipelineResource>,
    swapchain_surface_info: &SwapchainSurfaceInfo,
    depth_pyramid_pass: &DepthPyramidPass,
    mesh_culling_debug_data: Option<ModernPipelineMeshCullingDebugData>,
    mesh_culling_debug_output: RenderGraphExternalBufferId,
) -> MeshCullingPass {
    let invalid_image = context
        .render_resources
        .fetch::<InvalidResources>()
        .invalid_image_color
        .clone();

    let node = context
        .graph
        .add_node("MeshCulling", RenderGraphQueue::DefaultGraphics);

    let depth_pyramid_mips: Vec<_> = depth_pyramid_pass
        .depth_pyramid_mips
        .iter()
        .map(|x| {
            context.graph.sample_image(
                node,
                *x,
                RenderGraphImageConstraint {
                    ..Default::default()
                },
                Default::default(),
            )
        })
        .collect();

    let debug_output = context
        .graph
        .read_external_buffer(mesh_culling_debug_output);
    let debug_output = context.graph.modify_storage_buffer(
        node,
        debug_output,
        Default::default(),
        RafxLoadOp::Clear,
    );

    let mesh_culling_pipeline = mesh_culling_pipeline.clone();
    let swapchain_extents = swapchain_surface_info.extents;
    context.graph.set_callback(node, move |args| {
        let mut occlusion_jobs = args
            .graph_context
            .render_resources()
            .fetch_mut::<MeshAdvGpuOcclusionCullRenderResource>();
        for occlusion_job in &occlusion_jobs.data {
            let mut descriptor_set_allocator = args
                .graph_context
                .resource_context()
                .create_descriptor_set_allocator();
            let mut descriptor_set = descriptor_set_allocator
                .create_dyn_descriptor_set_uninitialized(
                    &mesh_culling_pipeline.get_raw().descriptor_set_layouts[0],
                )?;

            let debug_output = args.graph_context.buffer(debug_output).unwrap();

            let mut enable_debug_data_collection = false;

            // Copy the result of previous frame's histogram to debug data resource
            unsafe {
                let debug_output_ptr = debug_output.get_raw().buffer.map_buffer()?;
                let debug_output_ptr =
                    &*(debug_output_ptr as *mut mesh_culling_comp::DebugOutputBuffer);

                if let Some(mesh_culling_debug_data) = &mesh_culling_debug_data {
                    let mut guard = mesh_culling_debug_data.inner.lock().unwrap();
                    guard.culled_mesh_count = debug_output_ptr.culled_mesh_count;
                    guard.total_mesh_count = debug_output_ptr.total_mesh_count;
                    guard.culled_primitive_count = debug_output_ptr.culled_primitive_count;
                    guard.total_primitive_count = debug_output_ptr.total_primitive_count;
                    // println!("{:?}", *guard);

                    enable_debug_data_collection = guard.enable_debug_data_collection;
                }
                debug_output.get_raw().buffer.unmap_buffer()?;
            }

            descriptor_set.set_buffer_data(
                mesh_culling_comp::CONFIG_DESCRIPTOR_BINDING_INDEX as u32,
                &mesh_culling_comp::ConfigUniform {
                    view_matrix: occlusion_job.render_view.view_matrix().to_cols_array_2d(),
                    proj_matrix: occlusion_job
                        .render_view
                        .projection_matrix()
                        .to_cols_array_2d(),
                    draw_data_count: occlusion_job.draw_data_count,
                    indirect_first_command_index: occlusion_job.indirect_first_command_index,
                    depth_mip_slice_count: depth_pyramid_mips.len() as u32,
                    viewport_width: swapchain_extents.width,
                    viewport_height: swapchain_extents.height,
                    z_near: occlusion_job.render_view.depth_range().near,
                    write_debug_output: enable_debug_data_collection as u32,
                    _padding0: Default::default(),
                },
            );

            descriptor_set.set_buffer(
                mesh_culling_comp::ALL_DRAW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
                &occlusion_job.draw_data,
            );
            // descriptor_set.set_buffer(
            //     mesh_culling_comp::ALL_TRANSFORMS_DESCRIPTOR_BINDING_INDEX as u32,
            //     &occlusion_job.transforms,
            // );
            descriptor_set.set_buffer(
                mesh_culling_comp::ALL_BOUNDING_SPHERES_DESCRIPTOR_BINDING_INDEX as u32,
                &occlusion_job.bounding_spheres,
            );
            descriptor_set.set_buffer(
                mesh_culling_comp::ALL_INDIRECT_COMMANDS_DESCRIPTOR_BINDING_INDEX as u32,
                &occlusion_job.indirect_commands,
            );
            descriptor_set.set_buffer(
                mesh_culling_comp::DEBUG_OUTPUT_DESCRIPTOR_BINDING_INDEX as u32,
                &debug_output,
            );

            for (index, &depth_image) in depth_pyramid_mips.iter().enumerate() {
                let depth_image = args.graph_context.image_view(depth_image).unwrap();
                descriptor_set.set_image_at_index(
                    mesh_culling_comp::DEPTH_MIP_SLICES_DESCRIPTOR_BINDING_INDEX as u32,
                    index,
                    &depth_image,
                );
            }

            for i in depth_pyramid_mips.len()..(MAX_DEPTH_PYRAMID_MIP_LAYERS as usize) {
                descriptor_set.set_image_at_index(
                    mesh_culling_comp::DEPTH_MIP_SLICES_DESCRIPTOR_BINDING_INDEX as u32,
                    i,
                    &invalid_image,
                );
            }

            descriptor_set.flush(&mut descriptor_set_allocator)?;
            descriptor_set_allocator.flush_changes()?;

            // Draw calls
            let command_buffer = &args.command_buffer;

            command_buffer.cmd_bind_pipeline(&*mesh_culling_pipeline.get_raw().pipeline)?;
            descriptor_set.bind(command_buffer)?;

            let group_count = rafx::base::memory::round_size_up_to_alignment_u32(
                occlusion_job.draw_data_count,
                64,
            ) / 64;

            log::trace!(
                "culling for {} draws in {} groups command buffer offset {}",
                occlusion_job.draw_data_count,
                group_count,
                occlusion_job.indirect_first_command_index
            );
            command_buffer.cmd_dispatch(1, group_count, 1)?;

            // We need a manual barrier here because this resource is not managed by the render graph
            command_buffer.cmd_resource_barrier(
                &[RafxBufferBarrier {
                    buffer: &*occlusion_job.indirect_commands.get_raw().buffer,
                    src_state: RafxResourceState::UNORDERED_ACCESS,
                    dst_state: RafxResourceState::INDIRECT_ARGUMENT,
                    queue_transition: RafxBarrierQueueTransition::None,
                    offset_size: None,
                }],
                &[],
            )?;
        }

        occlusion_jobs.data.clear();

        Ok(())
    });

    MeshCullingPass { node }
}
