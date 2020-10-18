use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::{VkDeviceContext, MsaaLevel};
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::SwapchainInfo;
use renderer::vulkan::VkQueueFamilyIndices;
use crate::game_renderer::RenderpassAttachmentImage;
use renderer::assets::resources::{ResourceArc, ImageViewResource};

/// Draws sprites
pub struct VkMsaaRenderPass {
    pub device_context: VkDeviceContext,
    pub swapchain_info: SwapchainInfo,

    // Command pool and list of command buffers, one per present index
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,

    pub color_target_image: vk::Image,
    pub color_resolved_image: vk::Image,
}

impl VkMsaaRenderPass {
    pub fn new(
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        swapchain_images: &[ResourceArc<ImageViewResource>],
        color_attachment: &RenderpassAttachmentImage,
    ) -> VkResult<Self> {
        //
        // Command Buffers
        //
        let command_pool = Self::create_command_pool(
            &device_context.device(),
            &device_context.queue_family_indices(),
        )?;

        let command_buffers =
            Self::create_command_buffers(&device_context.device(), swapchain_info, &command_pool)?;

        let color_target_image = color_attachment.target_image();
        let color_resolved_image = color_attachment.resolved_image();

        Ok(VkMsaaRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain_info.clone(),
            command_pool,
            command_buffers,
            color_target_image,
            color_resolved_image,
        })
    }

    fn create_command_pool(
        logical_device: &ash::Device,
        queue_family_indices: &VkQueueFamilyIndices,
    ) -> VkResult<vk::CommandPool> {
        log::trace!(
            "Creating command pool with queue family index {}",
            queue_family_indices.graphics_queue_family_index
        );
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(
                vk::CommandPoolCreateFlags::TRANSIENT
                    | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            )
            .queue_family_index(queue_family_indices.graphics_queue_family_index);

        unsafe { logical_device.create_command_pool(&pool_create_info, None) }
    }

    fn create_command_buffers(
        logical_device: &ash::Device,
        swapchain_info: &SwapchainInfo,
        command_pool: &vk::CommandPool,
    ) -> VkResult<Vec<vk::CommandBuffer>> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(swapchain_info.image_count as u32)
            .command_pool(*command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info) }
    }

    #[allow(clippy::too_many_arguments)]
    fn update_command_buffer(
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        command_buffer: &vk::CommandBuffer,
        color_target_image: vk::Image,
        color_resolved_image: vk::Image,
    ) -> VkResult<()> {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();

        // Implicitly resets the command buffer
        unsafe {
            let logical_device = device_context.device();
            logical_device.begin_command_buffer(*command_buffer, &command_buffer_begin_info)?;

            if swapchain_info.msaa_level != MsaaLevel::Sample1 {
                Self::resolve_image(
                    &logical_device,
                    *command_buffer,
                    color_target_image,
                    color_resolved_image,
                    swapchain_info.extents,
                );
            }

            logical_device.end_command_buffer(*command_buffer)
        }
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
        present_index: usize,
        _descriptor_set_per_view: vk::DescriptorSet,
    ) -> VkResult<()> {
        //TODO: Can probably record these once and maybe even just have one
        Self::update_command_buffer(
            &self.device_context,
            &self.swapchain_info,
            &self.command_buffers[present_index],
            self.color_target_image,
            self.color_resolved_image,
        )
    }
}

impl Drop for VkMsaaRenderPass {
    fn drop(&mut self) {
        log::trace!("destroying VkSpriteRenderPass");
        unsafe {
            let device = self.device_context.device();

            device.destroy_command_pool(self.command_pool, None);
        }

        log::trace!("destroyed VkSpriteRenderPass");
    }
}
