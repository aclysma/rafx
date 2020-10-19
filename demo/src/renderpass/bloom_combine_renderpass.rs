use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::VkDeviceContext;
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::SwapchainInfo;
use renderer::vulkan::VkQueueFamilyIndices;

use renderer::assets::resources::{
    PipelineSwapchainInfo, ResourceArc, ImageViewResource, FramebufferResource, ResourceLookupSet,
    RenderPassResource, DynCommandWriter,
};
use renderer::assets::vk_description as dsc;

pub struct VkBloomCombineRenderPass {
    device_context: VkDeviceContext,
    swapchain_info: SwapchainInfo,
    pipeline_info: PipelineSwapchainInfo,
    frame_buffers: Vec<ResourceArc<FramebufferResource>>,
}

impl VkBloomCombineRenderPass {
    pub fn new(
        resources: &mut ResourceLookupSet,
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        swapchain_images: &[ResourceArc<ImageViewResource>],
        pipeline_info: PipelineSwapchainInfo,
    ) -> VkResult<Self> {
        let frame_buffers = Self::create_framebuffers(
            resources,
            swapchain_images,
            swapchain_info,
            &pipeline_info.pipeline.get_raw().renderpass,
        )?;

        Ok(VkBloomCombineRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain_info.clone(),
            pipeline_info,
            frame_buffers,
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
        descriptor_set: vk::DescriptorSet,
        command_writer: &mut DynCommandWriter,
    ) -> VkResult<vk::CommandBuffer> {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(
                self.pipeline_info
                    .pipeline
                    .get_raw()
                    .renderpass
                    .get_raw()
                    .renderpass,
            )
            .framebuffer(self.frame_buffers[present_index].get_raw().framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_info.extents,
            })
            .clear_values(&clear_values);

        let command_buffer = command_writer.begin_command_buffer(
            vk::CommandBufferLevel::PRIMARY,
            vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            None,
        )?;
        unsafe {
            let logical_device = self.device_context.device();
            logical_device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline.get_raw().pipelines[0],
            );

            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );

            logical_device.cmd_draw(command_buffer, 3, 1, 0, 0);

            logical_device.cmd_end_render_pass(command_buffer);
        }

        command_writer.end_command_buffer()
    }
}
