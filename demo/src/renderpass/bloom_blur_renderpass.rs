use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::VkDeviceContext;
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::SwapchainInfo;
use renderer::vulkan::VkQueueFamilyIndices;

use renderer::assets::resource_managers::PipelineSwapchainInfo;
use crate::renderpass::VkBloomRenderPassResources;

pub struct VkBloomBlurRenderPass {
    pub device_context: VkDeviceContext,
    pub swapchain_info: SwapchainInfo,

    pub frame_buffers: Vec<vk::Framebuffer>,

    // Command pool and list of command buffers. We ping-pong the blur filter, so there are two
    // command buffers, two framebuffers, two images, to descriptor sets, etc.
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
}

impl VkBloomBlurRenderPass {
    pub fn new(
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
        pipeline_info: PipelineSwapchainInfo,
        bloom_resources: &VkBloomRenderPassResources,
    ) -> VkResult<Self> {
        //
        // Command Buffers
        //
        let command_pool = Self::create_command_pool(
            &device_context.device(),
            &device_context.queue_family_indices(),
        )?;

        //
        // Renderpass Resources
        //
        let frame_buffers = Self::create_framebuffers(
            &device_context.device(),
            //&swapchain.swapchain_image_views,
            &bloom_resources.bloom_image_views,
            &swapchain.swapchain_info,
            &pipeline_info.pipeline.get_raw().renderpass.get_raw(),
        )?;

        let command_buffers = Self::create_command_buffers(
            &device_context.device(),
            &swapchain.swapchain_info,
            &command_pool,
        )?;

        let descriptor_set_per_pass0 = bloom_resources.bloom_image_descriptor_sets[0]
            .descriptor_set()
            .get();
        let descriptor_set_per_pass1 = bloom_resources.bloom_image_descriptor_sets[1]
            .descriptor_set()
            .get();

        Self::update_command_buffer(
            &device_context,
            &swapchain.swapchain_info,
            pipeline_info.pipeline.get_raw().renderpass.get_raw(),
            frame_buffers[1],
            command_buffers[0],
            pipeline_info.pipeline.get_raw().pipelines[0],
            pipeline_info.pipeline_layout.get_raw().pipeline_layout,
            descriptor_set_per_pass0,
        )?;

        Self::update_command_buffer(
            &device_context,
            &swapchain.swapchain_info,
            pipeline_info.pipeline.get_raw().renderpass.get_raw(),
            frame_buffers[0],
            command_buffers[1],
            pipeline_info.pipeline.get_raw().pipelines[0],
            pipeline_info.pipeline_layout.get_raw().pipeline_layout,
            descriptor_set_per_pass1,
        )?;

        Ok(VkBloomBlurRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain.swapchain_info.clone(),
            frame_buffers,
            command_pool,
            command_buffers,
            // bloom_image,
            // bloom_image_view
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
            .queue_family_index(queue_family_indices.graphics_queue_family_index);

        unsafe { logical_device.create_command_pool(&pool_create_info, None) }
    }

    fn create_framebuffers(
        logical_device: &ash::Device,
        bloom_image_views: &[vk::ImageView],
        swapchain_info: &SwapchainInfo,
        renderpass: &vk::RenderPass,
    ) -> VkResult<Vec<vk::Framebuffer>> {
        bloom_image_views
            .iter()
            .map(|&bloom_image_view| {
                let framebuffer_attachments = [bloom_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(*renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(swapchain_info.extents.width)
                    .height(swapchain_info.extents.height)
                    .layers(1);

                unsafe { logical_device.create_framebuffer(&frame_buffer_create_info, None) }
            })
            .collect()
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

    fn update_command_buffer(
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        renderpass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        command_buffer: vk::CommandBuffer,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
    ) -> VkResult<()> {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE);

        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(renderpass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_info.extents.clone(),
            })
            .clear_values(&clear_values);

        // Implicitly resets the command buffer
        unsafe {
            let logical_device = device_context.device();
            logical_device.begin_command_buffer(command_buffer, &command_buffer_begin_info)?;

            logical_device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline,
            );

            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );

            logical_device.cmd_draw(command_buffer, 3, 1, 0, 0);

            logical_device.cmd_end_render_pass(command_buffer);
            logical_device.end_command_buffer(command_buffer)
        }
    }
}

impl Drop for VkBloomBlurRenderPass {
    fn drop(&mut self) {
        log::trace!("destroying VkSpriteRenderPass");

        unsafe {
            let device = self.device_context.device();
            device.destroy_command_pool(self.command_pool, None);

            for frame_buffer in &self.frame_buffers {
                device.destroy_framebuffer(*frame_buffer, None);
            }
        }

        log::trace!("destroyed VkSpriteRenderPass");
    }
}
