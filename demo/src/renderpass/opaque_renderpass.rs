use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use renderer::vulkan::{VkDeviceContext, MAX_FRAMES_IN_FLIGHT};
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::VkQueueFamilyIndices;
use renderer::vulkan::SwapchainInfo;

use renderer::assets::resources::{
    PipelineSwapchainInfo, RenderPassResource, ResourceArc, ImageViewResource, FramebufferResource,
    ResourceLookupSet,
};
use renderer::assets::vk_description as dsc;
use renderer::nodes::{PreparedRenderData, RenderView};
use crate::phases::OpaqueRenderPhase;
use crate::render_contexts::{RenderJobWriteContext, RenderJobWriteContextFactory};
use renderer::vulkan::cleanup::VkCombinedDropSink;
use crate::game_renderer::RenderpassAttachmentImage;

/// Draws sprites
pub struct VkOpaqueRenderPass {
    pub device_context: VkDeviceContext,
    pub swapchain_info: SwapchainInfo,

    // Static resources for the renderpass, including a frame buffer per present index
    pub frame_buffers: Vec<ResourceArc<FramebufferResource>>,

    // Command pool and list of command buffers, one per present index
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,

    pub drop_sink: VkCombinedDropSink,

    renderpass: vk::RenderPass,
}

impl VkOpaqueRenderPass {
    pub fn new(
        resources: &mut ResourceLookupSet,
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        swapchain_images: &[ResourceArc<ImageViewResource>],
        color_attachment: &RenderpassAttachmentImage,
        depth_attachment: &RenderpassAttachmentImage,
        pipeline_info: PipelineSwapchainInfo,
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
            resources,
            color_attachment.target_resource(),
            &swapchain_images,
            depth_attachment.target_resource(),
            swapchain_info,
            &pipeline_info.pipeline.get_raw().renderpass,
        )?;

        let command_buffers =
            Self::create_command_buffers(&device_context.device(), swapchain_info, &command_pool)?;

        Ok(VkOpaqueRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain_info.clone(),
            frame_buffers,
            command_pool,
            command_buffers,
            renderpass: pipeline_info
                .pipeline
                .get_raw()
                .renderpass
                .get_raw()
                .renderpass,
            drop_sink: VkCombinedDropSink::new(MAX_FRAMES_IN_FLIGHT as u32),
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

    fn create_framebuffers(
        resources: &mut ResourceLookupSet,
        color_image_view: &ResourceArc<ImageViewResource>,
        swapchain_image_views: &[ResourceArc<ImageViewResource>],
        depth_image_view: &ResourceArc<ImageViewResource>,
        swapchain_info: &SwapchainInfo,
        renderpass: &ResourceArc<RenderPassResource>,
    ) -> VkResult<Vec<ResourceArc<FramebufferResource>>> {
        swapchain_image_views
            .iter()
            .map(|_swapchain_image_view| {
                let framebuffer_meta = dsc::FramebufferMeta {
                    width: swapchain_info.extents.width,
                    height: swapchain_info.extents.height,
                    layers: 1,
                };

                let attachments = [color_image_view.clone(), depth_image_view.clone()];
                resources.get_or_create_framebuffer(
                    renderpass.clone(),
                    &attachments,
                    &framebuffer_meta,
                )
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

    #[allow(clippy::too_many_arguments)]
    fn update_command_buffer(
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        renderpass: &vk::RenderPass,
        framebuffer: vk::Framebuffer,
        command_buffer: &vk::CommandBuffer,
        prepared_render_data: &PreparedRenderData<RenderJobWriteContext>,
        view: &RenderView,
        write_context_factory: &RenderJobWriteContextFactory,
    ) -> VkResult<()> {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();

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
            .render_pass(*renderpass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_info.extents,
            })
            .clear_values(&clear_values);

        // Implicitly resets the command buffer
        unsafe {
            let logical_device = device_context.device();
            logical_device.begin_command_buffer(*command_buffer, &command_buffer_begin_info)?;

            logical_device.cmd_begin_render_pass(
                *command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            let mut write_context = write_context_factory.create_context(*command_buffer);

            prepared_render_data.write_view_phase::<OpaqueRenderPhase>(&view, &mut write_context);

            logical_device.cmd_end_render_pass(*command_buffer);
            logical_device.end_command_buffer(*command_buffer)
        }
    }

    pub fn update(
        &mut self,
        pipeline_info: &PipelineSwapchainInfo,
        present_index: usize,
        prepared_render_data: &PreparedRenderData<RenderJobWriteContext>,
        view: &RenderView,
        write_context_factory: &RenderJobWriteContextFactory,
    ) -> VkResult<()> {
        assert!(
            self.renderpass
                == pipeline_info
                    .pipeline
                    .get_raw()
                    .renderpass
                    .get_raw()
                    .renderpass
        );
        Self::update_command_buffer(
            &self.device_context,
            &self.swapchain_info,
            &pipeline_info
                .pipeline
                .get_raw()
                .renderpass
                .get_raw()
                .renderpass,
            self.frame_buffers[present_index].get_raw().framebuffer,
            &self.command_buffers[present_index],
            prepared_render_data,
            view,
            write_context_factory,
        )
    }
}

impl Drop for VkOpaqueRenderPass {
    fn drop(&mut self) {
        log::trace!("destroying VkOpaqueRenderPass");

        unsafe {
            let device = self.device_context.device();

            device.destroy_command_pool(self.command_pool, None);
        }

        log::trace!("destroyed VkOpaqueRenderPass");
    }
}
