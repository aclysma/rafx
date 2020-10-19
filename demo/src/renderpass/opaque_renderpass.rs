use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::{VkDeviceContext, MAX_FRAMES_IN_FLIGHT};
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::VkQueueFamilyIndices;
use renderer::vulkan::SwapchainInfo;

use renderer::assets::resources::{
    PipelineSwapchainInfo, RenderPassResource, ResourceArc, ImageViewResource, FramebufferResource,
    ResourceLookupSet, DynCommandWriter,
};
use renderer::assets::vk_description as dsc;
use renderer::nodes::{PreparedRenderData, RenderView};
use crate::phases::OpaqueRenderPhase;
use crate::render_contexts::{RenderJobWriteContext, RenderJobWriteContextFactory};
use crate::game_renderer::RenderpassAttachmentImage;

/// Draws sprites
pub struct VkOpaqueRenderPass {
    device_context: VkDeviceContext,
    swapchain_info: SwapchainInfo,
    frame_buffer: ResourceArc<FramebufferResource>,
    renderpass: ResourceArc<RenderPassResource>,
}

impl VkOpaqueRenderPass {
    pub fn new(
        resources: &mut ResourceLookupSet,
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        color_attachment: &RenderpassAttachmentImage,
        depth_attachment: &RenderpassAttachmentImage,
        pipeline_info: PipelineSwapchainInfo,
    ) -> VkResult<Self> {
        let frame_buffer = Self::create_framebuffers(
            resources,
            color_attachment.target_resource(),
            depth_attachment.target_resource(),
            swapchain_info,
            &pipeline_info.pipeline.get_raw().renderpass,
        )?;

        Ok(VkOpaqueRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain_info.clone(),
            frame_buffer,
            renderpass: pipeline_info.pipeline.get_raw().renderpass.clone(),
        })
    }

    fn create_framebuffers(
        resources: &mut ResourceLookupSet,
        color_image_view: &ResourceArc<ImageViewResource>,
        depth_image_view: &ResourceArc<ImageViewResource>,
        swapchain_info: &SwapchainInfo,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> VkResult<ResourceArc<FramebufferResource>> {
        let framebuffer_meta = dsc::FramebufferMeta {
            width: swapchain_info.extents.width,
            height: swapchain_info.extents.height,
            layers: 1,
        };

        let attachments = [color_image_view.clone(), depth_image_view.clone()];
        resources.get_or_create_framebuffer(renderpass.clone(), &attachments, &framebuffer_meta)
    }

    pub fn update(
        &mut self,
        prepared_render_data: &PreparedRenderData<RenderJobWriteContext>,
        view: &RenderView,
        write_context_factory: &RenderJobWriteContextFactory,
        command_writer: &mut DynCommandWriter,
    ) -> VkResult<vk::CommandBuffer> {
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.renderpass.get_raw().renderpass)
            .framebuffer(self.frame_buffer.get_raw().framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_info.extents,
            })
            .clear_values(&clear_values);

        // Implicitly resets the command buffer
        unsafe {
            let logical_device = self.device_context.device();
            let command_buffer = command_writer.begin_command_buffer(
                vk::CommandBufferLevel::PRIMARY,
                vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                None,
            )?;

            logical_device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            let mut write_context = write_context_factory.create_context(command_buffer);

            prepared_render_data.write_view_phase::<OpaqueRenderPhase>(&view, &mut write_context);

            logical_device.cmd_end_render_pass(command_buffer);
            command_writer.end_command_buffer()
        }
    }
}
