use ash::vk;
use renderer::assets::vk_description as dsc;
use renderer::assets::graph::*;
use renderer::assets::resources::ResourceManager;
use crate::VkDeviceContext;
use ash::prelude::VkResult;
use renderer::assets::resources::{
    ResourceArc, ImageViewResource, DynCommandWriter, RenderPassResource,
};
use crate::render_contexts::{RenderJobWriteContextFactory, RenderJobWriteContext};
use renderer::nodes::{PreparedRenderData, RenderView};
use crate::phases::{OpaqueRenderPhase, UiRenderPhase};
use renderer::vulkan::SwapchainInfo;

pub struct BuildRenderGraphResult {
    pub opaque_renderpass: Option<ResourceArc<RenderPassResource>>,
    pub ui_renderpass: Option<ResourceArc<RenderPassResource>>,
    pub executor: RenderGraphExecutor<RenderGraphExecuteContext>,
}

// Any data you want available within rendergraph execution callbacks should go here. This can
// include data that is not known until later after the extract/prepare phases have completed.
pub struct RenderGraphExecuteContext {
    pub prepared_render_data: Box<PreparedRenderData<RenderJobWriteContext>>,
    pub view: RenderView,
    pub write_context_factory: RenderJobWriteContextFactory,
    pub command_writer: DynCommandWriter,
}

pub fn build_render_graph(
    swapchain_surface_info: &dsc::SwapchainSurfaceInfo,
    device_context: &VkDeviceContext,
    resource_manager: &mut ResourceManager,
    swapchain_info: &SwapchainInfo,
    swapchain_image: ResourceArc<ImageViewResource>,
) -> VkResult<BuildRenderGraphResult> {
    //TODO: Fix this back to be color format
    let color_format = swapchain_surface_info.surface_format.format;
    let depth_format = swapchain_surface_info.depth_format;
    let swapchain_format = swapchain_surface_info.surface_format.format;
    let samples = swapchain_surface_info.msaa_level.into();
    let queue = 0;

    let mut graph = RenderGraph::default();
    let mut graph_callbacks = RenderGraphNodeCallbacks::<RenderGraphExecuteContext>::default();

    let opaque_pass = {
        struct Opaque {
            node_id: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
            depth: RenderGraphImageUsageId,
        }

        let mut node = graph.add_node();
        let node_id = node.id();
        node.set_name("Opaque");
        let color = node.create_color_attachment(
            0,
            Some(vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            }),
            RenderGraphImageConstraint {
                samples: Some(samples),
                format: Some(color_format),
                ..Default::default()
            },
        );

        let depth = node.create_depth_attachment(
            Some(vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            }),
            RenderGraphImageConstraint {
                samples: Some(samples),
                format: Some(depth_format),
                queue: Some(queue),
                ..Default::default()
            },
        );

        graph_callbacks.add_renderphase_dependency::<OpaqueRenderPhase>(node.id());
        graph_callbacks.set_renderpass_callback(node.id(), |command_buffer, context| {
            let mut write_context = context.write_context_factory.create_context(command_buffer);
            context
                .prepared_render_data
                .write_view_phase::<OpaqueRenderPhase>(&context.view, &mut write_context);
            Ok(())
        });

        graph.configure_image(color).set_name("color");
        graph.configure_image(depth).set_name("depth");

        Opaque {
            node_id,
            color,
            depth,
        }
    };

    let ui_pass = {
        struct Ui {
            node_id: RenderGraphNodeId,
            color: RenderGraphImageUsageId,
        }

        let mut node = graph.add_node();
        let node_id = node.id();
        node.set_name("Ui");

        let color = node.modify_color_attachment(
            opaque_pass.color,
            0,
            RenderGraphImageConstraint {
                samples: Some(vk::SampleCountFlags::TYPE_1),
                ..Default::default()
            },
        );

        graph_callbacks.add_renderphase_dependency::<UiRenderPhase>(node.id());
        graph_callbacks.set_renderpass_callback(node.id(), |command_buffer, context| {
            let mut write_context = context.write_context_factory.create_context(command_buffer);
            context
                .prepared_render_data
                .write_view_phase::<UiRenderPhase>(&context.view, &mut write_context);
            Ok(())
        });

        Ui { node_id, color }
    };

    let _blur_extract_pass = {
        struct BlurExtractPass {
            node_id: RenderGraphNodeId,
            sdr_image: RenderGraphImageUsageId,
            hdr_image: RenderGraphImageUsageId,
        }

        let mut node = graph.add_node();
        let node_id = node.id();
        node.set_name("BlurExtract");

        //node.sample_image(ui_pass.node_id);
        let sdr_image = node.create_color_attachment(0, Default::default(), Default::default());
        let hdr_image = node.create_color_attachment(
            1,
            Some(vk::ClearColorValue::default()),
            RenderGraphImageConstraint {
                samples: Some(vk::SampleCountFlags::TYPE_1),
                format: Some(color_format),
                queue: Some(queue),
                ..Default::default() //aspect_flags: vk::ImageAspectFlags::
                                     //..Default::default()
            },
        );

        graph_callbacks.set_renderpass_callback(node.id(), |_command_buffer, _context| {
            // bind?

            Ok(())
        });

        BlurExtractPass {
            node_id,
            sdr_image,
            hdr_image,
        }
    };

    let _swapchain_output_image_id = graph
        .configure_image(ui_pass.color /*blur_extract_pass.sdr_image*/)
        .set_output_image(
            swapchain_image,
            RenderGraphImageSpecification {
                samples: vk::SampleCountFlags::TYPE_1,
                format: swapchain_format,
                queue,
                aspect_flags: vk::ImageAspectFlags::COLOR,
                usage_flags: swapchain_info.image_usage_flags,
            },
            dsc::ImageLayout::PresentSrcKhr,
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::empty(),
            vk::ImageAspectFlags::COLOR,
        );

    //
    // Create the executor, it needs to have access to the resource manager to add framebuffers
    // and renderpasses to the resource lookups
    //
    let executor = RenderGraphExecutor::new(
        &device_context,
        graph,
        resource_manager,
        swapchain_surface_info,
        graph_callbacks,
    )?;

    let opaque_renderpass = executor.renderpass_resource(opaque_pass.node_id);
    let ui_renderpass = executor.renderpass_resource(ui_pass.node_id);
    Ok(BuildRenderGraphResult {
        executor,
        opaque_renderpass,
        ui_renderpass,
    })
}
