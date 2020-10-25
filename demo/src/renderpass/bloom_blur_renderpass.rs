use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::VkDeviceContext;
use renderer::vulkan::SwapchainInfo;

use crate::renderpass::VkBloomRenderPassResources;
use renderer::assets::vk_description as dsc;
use renderer::assets::resources::{
    ResourceArc, ImageViewResource, ResourceLookupSet, RenderPassResource, FramebufferResource,
    CommandPool, GraphicsPipelineResource,
};

pub struct VkBloomBlurRenderPass {
    device_context: VkDeviceContext,
    swapchain_info: SwapchainInfo,

    frame_buffers: Vec<ResourceArc<FramebufferResource>>,

    // Command pool and list of command buffers. We ping-pong the blur filter, so there are two
    // command buffers, two framebuffers, two images, two descriptor sets, etc.
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub pipeline: ResourceArc<GraphicsPipelineResource>,
}

impl VkBloomBlurRenderPass {
    pub fn new(
        resources: &ResourceLookupSet,
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        pipeline: ResourceArc<GraphicsPipelineResource>,
        bloom_resources: &VkBloomRenderPassResources,
        static_command_pool: &mut CommandPool,
    ) -> VkResult<Self> {
        //
        // Renderpass Resources
        //
        let frame_buffers = Self::create_framebuffers(
            resources,
            &bloom_resources.bloom_images,
            swapchain_info,
            &pipeline.get_raw().renderpass,
        )?;

        let descriptor_set_per_pass0 = bloom_resources.bloom_image_descriptor_sets[0]
            .descriptor_set()
            .get();
        let descriptor_set_per_pass1 = bloom_resources.bloom_image_descriptor_sets[1]
            .descriptor_set()
            .get();

        let command_buffers =
            static_command_pool.create_command_buffers(vk::CommandBufferLevel::PRIMARY, 2)?;

        Self::record_command_buffer(
            &device_context,
            swapchain_info,
            pipeline.get_raw().renderpass.get_raw().renderpass,
            frame_buffers[1].get_raw().framebuffer,
            command_buffers[0],
            pipeline.get_raw().pipelines[0],
            pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
            descriptor_set_per_pass0,
        )?;

        Self::record_command_buffer(
            &device_context,
            swapchain_info,
            pipeline.get_raw().renderpass.get_raw().renderpass,
            frame_buffers[0].get_raw().framebuffer,
            command_buffers[1],
            pipeline.get_raw().pipelines[0],
            pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
            descriptor_set_per_pass1,
        )?;

        Ok(VkBloomBlurRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain_info.clone(),
            frame_buffers,
            command_buffers,
            pipeline,
        })
    }

    fn create_framebuffers(
        resources: &ResourceLookupSet,
        bloom_image_views: &[ResourceArc<ImageViewResource>],
        swapchain_info: &SwapchainInfo,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> VkResult<Vec<ResourceArc<FramebufferResource>>> {
        bloom_image_views
            .iter()
            .map(|bloom_image_view| {
                let framebuffer_meta = dsc::FramebufferMeta {
                    width: swapchain_info.extents.width,
                    height: swapchain_info.extents.height,
                    layers: 1,
                };

                let attachments = [bloom_image_view.clone()];
                resources.get_or_create_framebuffer(
                    renderpass.clone(),
                    &attachments,
                    &framebuffer_meta,
                )
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn record_command_buffer(
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
                extent: swapchain_info.extents,
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
