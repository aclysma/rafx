use std::mem;
use ash::vk;
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer::vulkan::{VkDevice, VkDeviceContext, MsaaLevel};
use renderer::vulkan::VkSwapchain;
use renderer::vulkan::offset_of;
use renderer::vulkan::SwapchainInfo;
use renderer::vulkan::VkQueueFamilyIndices;
use renderer::vulkan::VkBuffer;
use renderer::vulkan::util;

use renderer::vulkan::VkImage;
use image::error::ImageError::Decoding;
use image::{GenericImageView, ImageFormat};
use ash::vk::ShaderStageFlags;

use renderer::base::time::TimeState;
use renderer::resources::resource_managers::PipelineSwapchainInfo;
use crate::features::debug3d::LineList3D;

/// Per-pass "global" data
#[derive(Clone, Debug, Copy)]
struct DebugUniformBufferObject {
    // View and projection matrices
    view_proj: [[f32; 4]; 4],
}

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct DebugVertex {
    pub pos: [f32; 3],
    //pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

/// Draws sprites
pub struct VkDebugRenderPass {
    pub device_context: VkDeviceContext,
    pub swapchain_info: SwapchainInfo,

    pipeline_info: PipelineSwapchainInfo,

    pub frame_buffers: Vec<vk::Framebuffer>,

    // Command pool and list of command buffers, one per present index
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,

    pub color_target_image: vk::Image,
    pub color_resolved_image: vk::Image,
}

impl VkDebugRenderPass {
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
            swapchain.color_attachment.resolved_image_view(),
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

        let color_target_image = swapchain.color_attachment.target_image();
        let color_resolved_image = swapchain.color_attachment.resolved_image();

        Ok(VkDebugRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain.swapchain_info.clone(),
            pipeline_info,
            frame_buffers,
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

    fn create_framebuffers(
        logical_device: &ash::Device,
        color_target_image_view: vk::ImageView,
        color_resolved_image_view: vk::ImageView,
        swapchain_image_views: &[vk::ImageView],
        depth_image_view: vk::ImageView,
        swapchain_info: &SwapchainInfo,
        renderpass: &vk::RenderPass,
    ) -> VkResult<Vec<vk::Framebuffer>> {
        swapchain_image_views
            .iter()
            .map(|&swapchain_image_view| {
                let framebuffer_attachments = vec![color_target_image_view, depth_image_view];

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
        renderpass: &vk::RenderPass,
        framebuffer: vk::Framebuffer,
        pipeline: &vk::Pipeline,
        pipeline_layout: &vk::PipelineLayout,
        command_buffer: &vk::CommandBuffer,
        descriptor_set_per_view: &vk::DescriptorSet,
        //line_lists: Vec<LineList3D>,
        color_target_image: vk::Image,
        color_resolved_image: vk::Image,
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
                extent: swapchain_info.extents.clone(),
            })
            .clear_values(&clear_values);

        // Implicitly resets the command buffer
        unsafe {
            let logical_device = device_context.device();
            logical_device.begin_command_buffer(*command_buffer, &command_buffer_begin_info)?;

            // logical_device.cmd_begin_render_pass(
            //     *command_buffer,
            //     &render_pass_begin_info,
            //     vk::SubpassContents::INLINE,
            // );
            //
            // // // Used to be debug draw here
            // // //TODO: Combine this with opaque renderpass or make this a special MSAA pass
            // //
            // logical_device.cmd_end_render_pass(*command_buffer);

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
        descriptor_set_per_view: vk::DescriptorSet,
        //line_lists: Vec<LineList3D>,
    ) -> VkResult<()> {
        Self::update_command_buffer(
            &self.device_context,
            &self.swapchain_info,
            &self.pipeline_info.renderpass.get_raw(),
            self.frame_buffers[present_index],
            &self.pipeline_info.pipeline.get_raw().pipelines[0],
            &self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
            &self.command_buffers[present_index],
            &descriptor_set_per_view,
            //line_lists,
            self.color_target_image,
            self.color_resolved_image,
        )
    }
}

impl Drop for VkDebugRenderPass {
    fn drop(&mut self) {
        log::trace!("destroying VkSpriteRenderPass");

        fn drop_all_buffer_lists(buffer_list: &mut Vec<Vec<ManuallyDrop<VkBuffer>>>) {
            for buffers in buffer_list {
                for mut b in &mut *buffers {
                    unsafe {
                        ManuallyDrop::drop(&mut b);
                    }
                }
            }
        }

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
