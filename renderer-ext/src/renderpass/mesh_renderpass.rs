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
use ash::vk::ShaderStageFlags;

use crate::time::TimeState;

use crate::resource_managers::{PipelineSwapchainInfo, MeshInfo};
use crate::pipeline::gltf::MeshVertex;

/// Per-pass "global" data
#[derive(Clone, Debug, Copy)]
struct MeshUniformBufferObject {
    // View and projection matrices
    view_proj: [[f32; 4]; 4],
}

/// Draws sprites
pub struct VkMeshRenderPass {
    pub device_context: VkDeviceContext,
    pub swapchain_info: SwapchainInfo,

    // Static resources for the renderpass, including a frame buffer per present index
    pub frame_buffers: Vec<vk::Framebuffer>,

    // Command pool and list of command buffers, one per present index
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,

    renderpass: vk::RenderPass,
}

impl VkMeshRenderPass {
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
            &swapchain.swapchain_image_views,
            swapchain.depth_image_view,
            &swapchain.swapchain_info,
            &pipeline_info.renderpass.get_raw(),
        );

        let command_buffers = Self::create_command_buffers(
            &device_context.device(),
            &swapchain.swapchain_info,
            &command_pool,
        )?;

        Ok(VkMeshRenderPass {
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
        log::info!(
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
        swapchain_image_views: &Vec<vk::ImageView>,
        depth_image_view: vk::ImageView,
        swapchain_info: &SwapchainInfo,
        renderpass: &vk::RenderPass,
    ) -> Vec<vk::Framebuffer> {
        swapchain_image_views
            .iter()
            .map(|&swapchain_image_view| {
                let framebuffer_attachments = [swapchain_image_view, depth_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(*renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(swapchain_info.extents.width)
                    .height(swapchain_info.extents.height)
                    .layers(1);

                unsafe {
                    //TODO: Pass this error up
                    logical_device
                        .create_framebuffer(&frame_buffer_create_info, None)
                        .unwrap()
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
        framebuffer: &vk::Framebuffer,
        pipeline: &vk::Pipeline,
        pipeline_layout: &vk::PipelineLayout,
        command_buffer: &vk::CommandBuffer,
        descriptor_set_per_pass: &vk::DescriptorSet,
        descriptor_set_per_material: &[vk::DescriptorSet],
        descriptor_set_per_instance: &[vk::DescriptorSet],
        //meshes: &[Option<Mesh>], // loaded mesh?
        time_state: &TimeState,
        meshes: &[MeshInfo],
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
            .framebuffer(*framebuffer)
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

            if !meshes.is_empty() {
                logical_device.cmd_bind_pipeline(
                    *command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    *pipeline,
                );

                // Bind per-pass data (UBO with view/proj matrix, sampler)
                logical_device.cmd_bind_descriptor_sets(
                    *command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    *pipeline_layout,
                    0,
                    &[*descriptor_set_per_pass],
                    &[],
                );

                logical_device.cmd_bind_descriptor_sets(
                    *command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    *pipeline_layout,
                    1,
                    &[descriptor_set_per_material[0]],
                    &[],
                );

                logical_device.cmd_bind_descriptor_sets(
                    *command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    *pipeline_layout,
                    2,
                    &[descriptor_set_per_instance[0]],
                    &[],
                );

                for mesh in meshes {
                    for mesh_part in &mesh.mesh_asset.mesh_parts {
                        logical_device.cmd_bind_vertex_buffers(
                            *command_buffer,
                            0, // first binding
                            &[mesh.vertex_buffer.get_raw().buffer],
                            &[mesh_part.vertex_buffer_offset_in_bytes as u64], // offsets
                        );

                        logical_device.cmd_bind_index_buffer(
                            *command_buffer,
                            mesh.index_buffer.get_raw().buffer,
                            mesh_part.index_buffer_offset_in_bytes as u64, // offset
                            vk::IndexType::UINT16,
                        );

                        // // Bind per-draw-call data (i.e. texture)
                        // logical_device.cmd_bind_descriptor_sets(
                        //     *command_buffer,
                        //     vk::PipelineBindPoint::GRAPHICS,
                        //     *pipeline_layout,
                        //     1,
                        //     //&[/*descriptor_set_per_texture[mesh_part.image_handle as usize]*/ descriptor_set_per_texture[0]],
                        //     &[descr]
                        //     &[],
                        // );

                        //mesh_part.mesh_part.index_size[]

                        logical_device.cmd_draw_indexed(
                            *command_buffer,
                            mesh_part.index_buffer_size_in_bytes / 2, //sizeof(u16)
                            1,
                            0,
                            0,
                            0,
                        );
                    }
                }
            }

            logical_device.cmd_end_render_pass(*command_buffer);
            logical_device.end_command_buffer(*command_buffer)
        }
    }

    pub fn update(
        &mut self,
        pipeline_info: &PipelineSwapchainInfo,
        present_index: usize,
        hidpi_factor: f64,
        descriptor_set_per_pass: vk::DescriptorSet,
        descriptor_set_per_material: &[vk::DescriptorSet],
        descriptor_set_per_instance: &[vk::DescriptorSet],
        mesh_info: &[MeshInfo],
        time_state: &TimeState,
    ) -> VkResult<()> {
        assert!(self.renderpass == pipeline_info.renderpass.get_raw());
        Self::update_command_buffer(
            &self.device_context,
            &self.swapchain_info,
            &pipeline_info.renderpass.get_raw(),
            &self.frame_buffers[present_index],
            &pipeline_info.pipeline.get_raw().pipeline,
            &pipeline_info.pipeline_layout.get_raw().pipeline_layout,
            &self.command_buffers[present_index],
            &descriptor_set_per_pass,
            descriptor_set_per_material,
            descriptor_set_per_instance,
            time_state,
            mesh_info,
        )
    }
}

impl Drop for VkMeshRenderPass {
    fn drop(&mut self) {
        log::trace!("destroying VkMeshRenderPass");

        unsafe {
            let device = self.device_context.device();

            device.destroy_command_pool(self.command_pool, None);

            for frame_buffer in &self.frame_buffers {
                device.destroy_framebuffer(*frame_buffer, None);
            }
        }

        log::trace!("destroyed VkMeshRenderPass");
    }
}
