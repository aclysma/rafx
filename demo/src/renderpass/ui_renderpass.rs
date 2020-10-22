use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::{VkDeviceContext, MAX_FRAMES_IN_FLIGHT};
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::SwapchainInfo;
use renderer::vulkan::VkQueueFamilyIndices;

use renderer::assets::resources::{
    ResourceArc, ImageViewResource, ResourceLookupSet, RenderPassResource, FramebufferResource,
    DynCommandWriter,
};
use renderer::nodes::{PreparedRenderData, RenderView};
use crate::render_contexts::{RenderJobWriteContext, RenderJobWriteContextFactory};
use crate::phases::UiRenderPhase;
use renderer::assets::vk_description as dsc;

/// Draws sprites
pub struct VkUiRenderPass {
    device_context: VkDeviceContext,
    swapchain_info: SwapchainInfo,
    frame_buffers: Vec<ResourceArc<FramebufferResource>>,
    pub renderpass: ResourceArc<RenderPassResource>,
}

impl VkUiRenderPass {
    pub fn new(
        resources: &mut ResourceLookupSet,
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        swapchain_images: &[ResourceArc<ImageViewResource>],
        renderpass: ResourceArc<RenderPassResource>,
    ) -> VkResult<Self> {
        let frame_buffers =
            Self::create_framebuffers(resources, swapchain_images, swapchain_info, &renderpass)?;

        Ok(VkUiRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain_info.clone(),
            frame_buffers,
            renderpass,
        })
    }

    fn create_framebuffers(
        resources: &mut ResourceLookupSet,
        swapchain_image_views: &[ResourceArc<ImageViewResource>],
        swapchain_info: &SwapchainInfo,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> VkResult<Vec<ResourceArc<FramebufferResource>>> {
        swapchain_image_views
            .iter()
            .map(|swapchain_image_view| {
                let framebuffer_meta = dsc::FramebufferMeta {
                    width: swapchain_info.extents.width,
                    height: swapchain_info.extents.height,
                    layers: 1,
                };

                let attachments = [swapchain_image_view.clone()];
                resources.get_or_create_framebuffer(
                    renderpass.clone(),
                    &attachments,
                    &framebuffer_meta,
                )
            })
            .collect()
    }

    pub fn update(
        &mut self,
        present_index: usize,
        prepared_render_data: &PreparedRenderData<RenderJobWriteContext>,
        view: &RenderView,
        write_context_factory: &RenderJobWriteContextFactory,
        command_writer: &mut DynCommandWriter,
    ) -> VkResult<vk::CommandBuffer> {
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.renderpass.get_raw().renderpass)
            .framebuffer(self.frame_buffers[present_index].get_raw().framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_info.extents,
            });

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

            prepared_render_data.write_view_phase::<UiRenderPhase>(&view, &mut write_context);

            logical_device.cmd_end_render_pass(command_buffer);
            command_writer.end_command_buffer()
        }
    }
}
