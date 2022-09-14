use rafx::framework::DescriptorSetBindings;
use rafx::graph::*;

use super::ModernPipelineContext;
use crate::features::mesh_adv::light_binning::MeshAdvLightBinRenderResource;
use crate::features::mesh_adv::MeshAdvStaticResources;
use crate::shaders::mesh_adv::{lights_bin_comp, lights_build_lists_comp};
use rafx::api::{RafxLoadOp, RafxResourceState};

pub struct LightBinPass {
    pub bitfields_buffer: RenderGraphBufferUsageId,
}

pub(super) fn lights_bin_pass(context: &mut ModernPipelineContext) -> LightBinPass {
    //
    // Rebuild the frustum AABB structure if the projection matrix has changed
    //
    let mut light_bin_render_resource = context
        .render_resources
        .fetch_mut::<MeshAdvLightBinRenderResource>();
    light_bin_render_resource
        .update_projection(
            context.resource_context,
            &context.main_view.projection_matrix(),
        )
        .unwrap();

    //
    // Get the compute pipeline
    //
    let static_resources = context.render_resources.fetch::<MeshAdvStaticResources>();
    let lights_bin_pipeline = context
        .asset_manager
        .committed_asset(&static_resources.lights_bin_compute_pipeline)
        .unwrap()
        .compute_pipeline
        .clone();

    //
    // Get the external clusters buffer
    //
    let clusters_buffer = light_bin_render_resource
        .frustum_bounds_gpu_buffer()
        .clone()
        .unwrap();
    let clusters_buffer = context.graph.add_external_buffer(
        clusters_buffer,
        RafxResourceState::SHADER_RESOURCE,
        RafxResourceState::SHADER_RESOURCE,
    );
    let clusters_buffer = context.graph.read_external_buffer(clusters_buffer);

    //
    // Get the external lights buffer
    //
    let lights_buffer = light_bin_render_resource
        .light_bounds_gpu_buffer(context.main_view.frame_index())
        .clone();
    let lights_buffer = context.graph.add_external_buffer(
        lights_buffer,
        RafxResourceState::SHADER_RESOURCE,
        RafxResourceState::SHADER_RESOURCE,
    );
    let lights_buffer = context.graph.read_external_buffer(lights_buffer);

    //
    // Setup the node
    //
    let node = context
        .graph
        .add_callback_node("LightsBin", RenderGraphQueue::DefaultGraphics);

    let clusters_buffer =
        context
            .graph
            .read_storage_buffer(node, clusters_buffer, Default::default());
    let lights_buffer = context
        .graph
        .read_storage_buffer(node, lights_buffer, Default::default());
    let lights_bin_bitfields_buffer = context.graph.create_storage_buffer(
        node,
        RenderGraphBufferConstraint {
            size: Some(std::mem::size_of::<lights_bin_comp::LightBitfieldsBuffer>() as u64),
            ..Default::default()
        },
        RafxLoadOp::Clear,
    );

    context.graph.set_callback(node, move |args| {
        let clusters_buffer = args.graph_context.buffer(clusters_buffer).unwrap();
        let lights_buffer = args.graph_context.buffer(lights_buffer).unwrap();
        let bitfield_buffer = args
            .graph_context
            .buffer(lights_bin_bitfields_buffer)
            .unwrap();

        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();
        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &lights_bin_pipeline.get_raw().descriptor_set_layouts[0],
        )?;

        descriptor_set.set_buffer(
            lights_bin_comp::CONFIG_DESCRIPTOR_BINDING_INDEX as u32,
            &clusters_buffer,
        );
        descriptor_set.set_buffer(
            lights_bin_comp::LIGHTS_DESCRIPTOR_BINDING_INDEX as u32,
            &lights_buffer,
        );
        descriptor_set.set_buffer(
            lights_bin_comp::BITFIELDS_DESCRIPTOR_BINDING_INDEX as u32,
            &bitfield_buffer,
        );

        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;

        command_buffer.cmd_bind_pipeline(&*lights_bin_pipeline.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;
        //X = cluster count/workgroup size
        //Y = light count/32/workgroup size (we process 32 lights per invocation, 1 for each u32 bit)
        //TODO: Can we dispatch (X=1024, Y=1) instead so that we dispatch less groups when we have fewer lights?
        command_buffer.cmd_dispatch((8 * 16 * 24) / 64, 1, 1)?;

        Ok(())
    });

    LightBinPass {
        bitfields_buffer: lights_bin_bitfields_buffer,
    }
}

pub struct LightBuildListsPass {
    pub light_lists_buffer: RenderGraphBufferUsageId,
}

pub(super) fn lights_build_lists_pass(
    context: &mut ModernPipelineContext,
    light_bin_pass: LightBinPass,
) -> LightBuildListsPass {
    // Get the compute pipeline
    let static_resources = context.render_resources.fetch::<MeshAdvStaticResources>();
    let lights_build_lists_pipeline = context
        .asset_manager
        .committed_asset(&static_resources.lights_build_lists_compute_pipeline)
        .unwrap()
        .compute_pipeline
        .clone();

    let light_bin_render_resource = context
        .render_resources
        .fetch::<MeshAdvLightBinRenderResource>();

    let node = context
        .graph
        .add_callback_node("LightsBuildLists", RenderGraphQueue::DefaultGraphics);

    let input_buffer = context.graph.read_storage_buffer(
        node,
        light_bin_pass.bitfields_buffer,
        Default::default(),
    );

    // OUTPUT
    let output_buffer = light_bin_render_resource
        .output_gpu_buffer(context.main_view.frame_index())
        .clone();
    let output_buffer = context.graph.add_external_buffer(
        output_buffer,
        RafxResourceState::SHADER_RESOURCE,
        RafxResourceState::SHADER_RESOURCE,
    );
    let output_buffer = context.graph.read_external_buffer(output_buffer);
    let output_buffer = context.graph.modify_storage_buffer(
        node,
        output_buffer,
        Default::default(),
        RafxLoadOp::Clear,
    );

    context.graph.set_callback(node, move |args| {
        let input_buffer = args.graph_context.buffer(input_buffer).unwrap();
        let output_buffer = args.graph_context.buffer(output_buffer).unwrap();

        let mut descriptor_set_allocator = args
            .graph_context
            .resource_context()
            .create_descriptor_set_allocator();
        let mut descriptor_set = descriptor_set_allocator.create_dyn_descriptor_set_uninitialized(
            &lights_build_lists_pipeline.get_raw().descriptor_set_layouts[0],
        )?;

        descriptor_set.set_buffer(
            lights_build_lists_comp::INPUT_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            &input_buffer,
        );
        descriptor_set.set_buffer(
            lights_build_lists_comp::OUTPUT_DATA_DESCRIPTOR_BINDING_INDEX as u32,
            &output_buffer,
        );

        descriptor_set.flush(&mut descriptor_set_allocator)?;
        descriptor_set_allocator.flush_changes()?;

        // Draw calls
        let command_buffer = &args.command_buffer;

        command_buffer.cmd_bind_pipeline(&*lights_build_lists_pipeline.get_raw().pipeline)?;
        descriptor_set.bind(command_buffer)?;

        // X = cluster count/workgroup size
        command_buffer.cmd_dispatch((8 * 16 * 24) / 1024, 1, 1)?;

        Ok(())
    });

    LightBuildListsPass {
        light_lists_buffer: output_buffer,
    }
}
