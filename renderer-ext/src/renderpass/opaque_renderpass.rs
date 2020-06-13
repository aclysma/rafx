use std::mem;
use ash::vk;
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{VkDevice, VkDeviceContext};
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use std::process::exit;
use image::{GenericImageView, ImageFormat};
use ash::vk::{ShaderStageFlags};

use crate::time::TimeState;

use crate::resource_managers::{PipelineSwapchainInfo, MeshInfo, DynDescriptorSet, ResourceManager};
use crate::pipeline::gltf::{MeshVertex, MeshAsset};
use crate::pipeline::pipeline::MaterialAsset;
use atelier_assets::loader::handle::Handle;
use crate::asset_resource::AssetResource;
use renderer_base::{PreparedRenderData, RenderView};
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use crate::CommandWriterContext;

/// Draws sprites
pub struct VkOpaqueRenderPass {
    pub device_context: VkDeviceContext,
    pub swapchain_info: SwapchainInfo,

    // Static resources for the renderpass, including a frame buffer per present index
    pub frame_buffers: Vec<vk::Framebuffer>,

    // Command pool and list of command buffers, one per present index
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,

    renderpass: vk::RenderPass,
}

impl VkOpaqueRenderPass {
    pub fn new(
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
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
            &device_context.device(),
            swapchain.color_attachment.target_image_view(),
            &swapchain.swapchain_image_views,
            swapchain.depth_attachment.target_image_view(),
            &swapchain.swapchain_info,
            &pipeline_info.renderpass.get_raw(),
        )?;

        let command_buffers = Self::create_command_buffers(
            &device_context.device(),
            &swapchain.swapchain_info,
            &command_pool,
        )?;

        Ok(VkOpaqueRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain.swapchain_info.clone(),
            frame_buffers,
            command_pool,
            command_buffers,
            renderpass: pipeline_info.renderpass.get_raw()
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
        logical_device: &ash::Device,
        color_image_view: vk::ImageView,
        swapchain_image_views: &[vk::ImageView],
        depth_image_view: vk::ImageView,
        swapchain_info: &SwapchainInfo,
        renderpass: &vk::RenderPass,
    ) -> VkResult<Vec<vk::Framebuffer>> {
        swapchain_image_views
            .iter()
            .map(|&swapchain_image_view| {
                let framebuffer_attachments = [color_image_view, depth_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(*renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(swapchain_info.extents.width)
                    .height(swapchain_info.extents.height)
                    .layers(1);

                unsafe {
                    logical_device
                        .create_framebuffer(&frame_buffer_create_info, None)
                }
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
        renderpass: &vk::RenderPass,
        framebuffer: vk::Framebuffer,
        command_buffer: &vk::CommandBuffer,
        prepared_render_data: &PreparedRenderData<CommandWriterContext>,
        view: &RenderView,
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
                    stencil: 0
                }
            }
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(*renderpass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: swapchain_info.extents.clone(),
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

            let mut write_context = CommandWriterContext {};
            prepared_render_data
                .write_view_phase::<DrawOpaqueRenderPhase>(&view, &mut write_context);

            logical_device.cmd_end_render_pass(*command_buffer);
            logical_device.end_command_buffer(*command_buffer)
        }
    }

    pub fn update(
        &mut self,
        pipeline_info: &PipelineSwapchainInfo,
        present_index: usize,
        prepared_render_data: &PreparedRenderData<CommandWriterContext>,
        view: &RenderView,
    ) -> VkResult<()> {
        assert!(self.renderpass == pipeline_info.renderpass.get_raw());
        Self::update_command_buffer(
            &self.device_context,
            &self.swapchain_info,
            &pipeline_info.renderpass.get_raw(),
            self.frame_buffers[present_index],
            &self.command_buffers[present_index],
            prepared_render_data,
            view
        )
    }
}

impl Drop for VkOpaqueRenderPass {
    fn drop(&mut self) {
        log::trace!("destroying VkOpaqueRenderPass");

        unsafe {
            let device = self.device_context.device();

            device.destroy_command_pool(self.command_pool, None);

            for frame_buffer in &self.frame_buffers {
                device.destroy_framebuffer(*frame_buffer, None);
            }
        }

        log::trace!("destroyed VkOpaqueRenderPass");
    }
}
