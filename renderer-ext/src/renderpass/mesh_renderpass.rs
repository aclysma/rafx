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


// Represents the data uploaded to the GPU to represent a single point light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct PointLight {
    pub position_ws: glam::Vec3, // +0
    pub position_vs: glam::Vec3, // +16
    pub color: glam::Vec4, // +32
    pub range: f32, // +48
    pub intensity: f32, // +52
} // 4*16 = 64 bytes

// Represents the data uploaded to the GPU to represent a single directional light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct DirectionalLight {
    pub direction_ws: glam::Vec3, // +0
    pub direction_vs: glam::Vec3, // +16
    pub color: glam::Vec4, // +32
    pub intensity: f32, // +48
} // 4*16 = 64 bytes

// Represents the data uploaded to the GPU to represent a single spot light
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct SpotLight {
    pub position_ws: glam::Vec3, // +0
    pub direction_ws: glam::Vec3, // +16
    pub position_vs: glam::Vec3, // +32
    pub direction_vs: glam::Vec3, // +48
    pub color: glam::Vec4, // +64
    pub spotlight_half_angle: f32, //+80
    pub range: f32, // +84
    pub intensity: f32, // +88
} // 6*16 = 96 bytes

// Represents the data uploaded to the GPU to provide all data necessary to render meshes
//TODO: Remove view/proj, they aren't being used. Add ambient light constant
#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct PerFrameDataShaderParam {
    pub ambient_light: glam::Vec4, // +0
    pub point_light_count: u32, // +16
    pub directional_light_count: u32, // 20
    pub spot_light_count: u32, // +24
    pub point_lights: [PointLight; 16], // +32 (64*16 = 1024),
    pub directional_lights: [DirectionalLight; 16], // +1056 (64*16 = 1024),
    pub spot_lights: [SpotLight; 16], // +2080 (96*16 = 1536)
} // 3616 bytes

#[derive(Default, Copy, Clone)]
#[repr(C)]
pub struct PerObjectDataShaderParam {
    pub model_view: glam::Mat4, // +0
    pub model_view_proj: glam::Mat4, // +64
} // 128 bytes


// A mesh that, aside from moving around, does not change. (i.e. no material changes)
pub struct StaticMeshInstance {
    // Contains buffers, where to bind within the buffers
    pub mesh_info: MeshInfo,

    // Dynamic descriptor for position/view. These are bound to layout 2.
    // These really should be per-view so there probably needs to be a better way of handling this
    pub per_object_descriptor_set: DynDescriptorSet,

    // world-space transform (position/rotation/translation)
    pub world_transform: glam::Mat4,
}

impl StaticMeshInstance {
    pub fn new(
        resource_manager: &mut ResourceManager,
        mesh: &Handle<MeshAsset>,
        mesh_material: &Handle<MaterialAsset>,
        position: glam::Vec3,
    ) -> VkResult<Self> {
        let mesh_info = resource_manager.get_mesh_info(mesh);
        let object_descriptor_set = resource_manager.get_descriptor_set_info(mesh_material, 0, 2);
        let per_object_descriptor_set = resource_manager.create_dyn_descriptor_set_uninitialized(&object_descriptor_set.descriptor_set_layout_def)?;

        let world_transform = glam::Mat4::from_translation(position);

        Ok(StaticMeshInstance {
            mesh_info,
            per_object_descriptor_set,
            world_transform
        })
    }

    pub fn set_view_proj(
        &mut self,
        view: glam::Mat4,
        proj: glam::Mat4,
    ) {
        let model_view = view * self.world_transform;
        let model_view_proj = proj * model_view;

        let per_object_param = PerObjectDataShaderParam {
            model_view,
            model_view_proj
        };

        self.per_object_descriptor_set.set_buffer_data(0, &per_object_param);
        self.per_object_descriptor_set.flush();
    }
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
            swapchain.color_image_view,
            &swapchain.swapchain_image_views,
            swapchain.depth_image_view,
            &swapchain.swapchain_info,
            &pipeline_info.renderpass.get_raw(),
        )?;

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
        pipeline: &vk::Pipeline,
        pipeline_layout: &vk::PipelineLayout,
        command_buffer: &vk::CommandBuffer,
        descriptor_set_per_pass: vk::DescriptorSet,
        meshes: &[StaticMeshInstance],
        asset_resource: &AssetResource,
        resource_manager: &ResourceManager,
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
                    &[descriptor_set_per_pass],
                    &[],
                );

                // logical_device.cmd_bind_descriptor_sets(
                //     *command_buffer,
                //     vk::PipelineBindPoint::GRAPHICS,
                //     *pipeline_layout,
                //     1,
                //     &[descriptor_set_per_material[0]],
                //     &[],
                // );
                //
                // logical_device.cmd_bind_descriptor_sets(
                //     *command_buffer,
                //     vk::PipelineBindPoint::GRAPHICS,
                //     *pipeline_layout,
                //     2,
                //     &[descriptor_set_per_instance[0]],
                //     &[],
                // );

                for mesh in meshes {
                    let per_object_descriptor_set = mesh.per_object_descriptor_set.descriptor_set().get_raw_for_gpu_read(resource_manager);

                    logical_device.cmd_bind_descriptor_sets(
                        *command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        *pipeline_layout,
                        2,
                        &[per_object_descriptor_set],
                        &[],
                    );

                    for mesh_part in &mesh.mesh_info.mesh_asset.mesh_parts {

                        let material_descriptor_set = resource_manager
                            .get_material_instance_descriptor_sets_for_current_frame(&mesh_part.material_instance, 0).descriptor_sets[1];

                        logical_device.cmd_bind_descriptor_sets(
                            *command_buffer,
                            vk::PipelineBindPoint::GRAPHICS,
                            *pipeline_layout,
                            1,
                            &[material_descriptor_set],
                            &[],
                        );

                        logical_device.cmd_bind_vertex_buffers(
                            *command_buffer,
                            0, // first binding
                            &[mesh.mesh_info.vertex_buffer.get_raw().buffer],
                            &[mesh_part.vertex_buffer_offset_in_bytes as u64], // offsets
                        );

                        logical_device.cmd_bind_index_buffer(
                            *command_buffer,
                            mesh.mesh_info.index_buffer.get_raw().buffer,
                            mesh_part.index_buffer_offset_in_bytes as u64, // offset
                            vk::IndexType::UINT16,
                        );

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
        descriptor_set_per_pass: vk::DescriptorSet,
        meshes: &[StaticMeshInstance],
        asset_resource: &AssetResource,
        resource_manager: &ResourceManager,
        // descriptor_set_per_material: &[vk::DescriptorSet],
        // descriptor_set_per_instance: &[vk::DescriptorSet],
    ) -> VkResult<()> {
        assert!(self.renderpass == pipeline_info.renderpass.get_raw());
        Self::update_command_buffer(
            &self.device_context,
            &self.swapchain_info,
            &pipeline_info.renderpass.get_raw(),
            self.frame_buffers[present_index],
            &pipeline_info.pipeline.get_raw().pipeline,
            &pipeline_info.pipeline_layout.get_raw().pipeline_layout,
            &self.command_buffers[present_index],
            descriptor_set_per_pass,
            meshes,
            asset_resource,
            resource_manager
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
