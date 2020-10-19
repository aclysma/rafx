use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::{VkDeviceContext, MsaaLevel};
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::SwapchainInfo;
use renderer::vulkan::VkQueueFamilyIndices;
use crate::game_renderer::RenderpassAttachmentImage;
use renderer::assets::resources::{ResourceArc, ImageViewResource, DynCommandWriter};

/// Draws sprites
pub struct VkMsaaRenderPass {
    device_context: VkDeviceContext,
    swapchain_info: SwapchainInfo,
    color_target_image: vk::Image,
    color_resolved_image: vk::Image,
}

impl VkMsaaRenderPass {
    pub fn new(
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        color_attachment: &RenderpassAttachmentImage,
    ) -> VkResult<Self> {
        let color_target_image = color_attachment.target_image();
        let color_resolved_image = color_attachment.resolved_image();

        Ok(VkMsaaRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain_info.clone(),
            color_target_image,
            color_resolved_image,
        })
    }

    unsafe fn resolve_image(
        logical_device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        color_target_image: vk::Image,
        color_resolved_image: vk::Image,
        image_extents: vk::Extent2D,
    ) {
        // Convert output of renderpass from SHADER_READ_ONLY_OPTIMAL to TRANSFER_SRC_OPTIMAL
        Self::add_image_barrier(
            logical_device,
            command_buffer,
            color_target_image,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        );

        // Convert output of resolve from UNDEFINED to TRANSFER_DST_OPTIMAL
        Self::add_image_barrier(
            logical_device,
            command_buffer,
            color_resolved_image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        // Specify that we are resolving the entire image
        let subresource_layers = ash::vk::ImageSubresourceLayers::builder()
            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1);
        let offset = vk::Offset3D::builder();
        let extent = vk::Extent3D::builder()
            .width(image_extents.width)
            .height(image_extents.height)
            .depth(1);
        let image_resolve = ash::vk::ImageResolve::builder()
            .src_subresource(*subresource_layers)
            .src_offset(*offset)
            .dst_subresource(*subresource_layers)
            .dst_offset(*offset)
            .extent(*extent);

        // Do the resolve
        logical_device.cmd_resolve_image(
            command_buffer,
            color_target_image,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            color_resolved_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[*image_resolve],
        );

        // Next usage of the renderpass output image will read it as undefined, so we don't need
        // to convert back

        // Convert the resolved output image to SHADER_READ_ONLY_OPTIMAL
        Self::add_image_barrier(
            logical_device,
            command_buffer,
            color_resolved_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
    }

    unsafe fn add_image_barrier(
        logical_device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let subresource_range = ash::vk::ImageSubresourceRange::builder()
            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        let image_memory_barrier = ash::vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(ash::vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(ash::vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(*subresource_range);

        logical_device.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::BY_REGION,
            &[],
            &[],
            &[*image_memory_barrier],
        );
    }

    pub fn update(
        &mut self,
        command_writer: &mut DynCommandWriter,
    ) -> VkResult<vk::CommandBuffer> {
        unsafe {
            let command_buffer = command_writer.begin_command_buffer(
                vk::CommandBufferLevel::PRIMARY,
                vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                None,
            )?;

            if self.swapchain_info.msaa_level != MsaaLevel::Sample1 {
                Self::resolve_image(
                    &self.device_context.device(),
                    command_buffer,
                    self.color_target_image,
                    self.color_resolved_image,
                    self.swapchain_info.extents,
                );
            }

            command_writer.end_command_buffer()
        }
    }
}
