use std::mem;
use ash::vk;
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{VkDevice, VkDeviceContext, MsaaLevel};
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use image::{GenericImageView, ImageFormat};
use ash::vk::ShaderStageFlags;

use crate::time::TimeState;
use crate::pipeline_description::{AttachmentReference, SwapchainSurfaceInfo};
use crate::asset_resource::AssetResource;
use crate::resource_managers::PipelineSwapchainInfo;


pub struct LineList3D {
    pub points: Vec<glam::Vec3>,
    pub color: glam::Vec4,
}

impl LineList3D {
    pub fn new(
        points: Vec<glam::Vec3>,
        color: glam::Vec4,
    ) -> Self {
        LineList3D { points, color }
    }
}

pub struct DebugDraw3DResource {
    line_lists: Vec<LineList3D>,
}

impl DebugDraw3DResource {
    pub fn new() -> Self {
        DebugDraw3DResource { line_lists: vec![] }
    }

    pub fn add_line_strip(
        &mut self,
        mut points: Vec<glam::Vec3>,
        color: glam::Vec4,
    ) {
        // Nothing will draw if we don't have at least 2 points
        if points.len() > 1 {
            self.line_lists.push(LineList3D::new(points, color));
        }
    }

    // Adds a single polygon
    pub fn add_line_loop(
        &mut self,
        mut points: Vec<glam::Vec3>,
        color: glam::Vec4,
    ) {
        // Nothing will draw if we don't have at least 2 points
        if points.len() > 1 {
            points.push(points[0].clone());
            self.add_line_strip(points, color);
        }
    }

    pub fn add_line(
        &mut self,
        p0: glam::Vec3,
        p1: glam::Vec3,
        color: glam::Vec4,
    ) {
        let points = vec![p0, p1];
        self.add_line_strip(points, color);
    }

    // Takes an X/Y axis pair and center position
    pub fn add_circle_xy(
        &mut self,
        center: glam::Vec3,
        x_dir: glam::Vec3,
        y_dir: glam::Vec3,
        radius: f32,
        color: glam::Vec4,
        segments: u32,
    ) {
        let x_dir = x_dir * radius;
        let y_dir = y_dir * radius;

        let mut points = Vec::with_capacity(segments as usize + 1);
        for index in 0..segments {
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;

            //let position = glam::Vec4::new(fraction.sin() * radius, fraction.cos() * radius, 0.0, 1.0);
            //let transformed = transform * position;
            points.push(center + (fraction.cos() * x_dir) + (fraction.sin() * y_dir));
        }

        self.add_line_loop(points, color);
    }

    pub fn normal_to_xy(normal: glam::Vec3) -> (glam::Vec3, glam::Vec3) {
        if normal.dot(glam::Vec3::unit_z()).abs() > 0.9999 {
            // Can't cross the Z axis with the up vector, so special case that here
            (glam::Vec3::unit_x(), glam::Vec3::unit_y())
        } else {
            let x_dir = normal.cross(glam::Vec3::unit_z());
            let y_dir = x_dir.cross(normal);
            (x_dir, y_dir)
        }
    }

    // Takes a normal and center position
    pub fn add_circle(
        &mut self,
        center: glam::Vec3,
        normal: glam::Vec3,
        radius: f32,
        color: glam::Vec4,
        segments: u32,
    ) {
        let (x_dir, y_dir) = Self::normal_to_xy(normal);
        self.add_circle_xy(center, x_dir, y_dir, radius, color, segments);
    }

    pub fn add_sphere(
        &mut self,
        center: glam::Vec3,
        radius: f32,
        color: glam::Vec4,
        segments: u32
    ) {
        let world_tranform = glam::Mat4::from_translation(center);

        // Draw the vertical rings
        for index in 0..segments {
            // Rotate around whole sphere (2pi)
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;
            let x_dir = glam::Vec3::new(fraction.cos(), fraction.sin(), 0.0);
            let y_dir = glam::Vec3::unit_z();

            self.add_circle_xy(
                center,
                x_dir,
                y_dir,
                radius,
                color,
                segments
            );
        }

        // Draw the center horizontal ring
        self.add_circle_xy(
            center,
            glam::Vec3::unit_x(),
            glam::Vec3::unit_y(),
            radius,
            color,
            segments
        );

        // Draw the off-center horizontal rings
        for index in 1..(segments / 2) {
            let fraction = (index as f32 / segments as f32) * std::f32::consts::PI * 2.0;

            let r = radius * fraction.cos();
            let z_offset = radius * fraction.sin() * glam::Vec3::unit_z();

            //let transform = glam::Mat4::from_translation(center + glam::Vec3::new(0.0, 0.0, z_offset));
            self.add_circle_xy(
                center + z_offset,
                glam::Vec3::unit_x(),
                glam::Vec3::unit_y(),
                r,
                color,
                segments
            );

            self.add_circle_xy(
                center - z_offset,
                glam::Vec3::unit_x(),
                glam::Vec3::unit_y(),
                r,
                color,
                segments
            );
        }
    }

    pub fn add_cone(
        &mut self,
        vertex: glam::Vec3, // (position of the pointy bit)
        base_center: glam::Vec3, // (position of the center of the base of the cone)
        radius: f32,
        color: glam::Vec4,
        segments: u32,
    ) {
        let base_to_vertex = vertex - base_center;
        let base_to_vertex_normal = base_to_vertex.normalize();
        let (x_dir, y_dir) = Self::normal_to_xy(base_to_vertex_normal);
        for index in 0..segments {
            let fraction = (index as f32 / segments as f32);

            let center = base_center + base_to_vertex * fraction;
            self.add_circle_xy(center, x_dir, y_dir, radius * (1.0 - fraction), color, segments);
        }

        for index in 0..segments/2 {
            let fraction = (index as f32 / (segments/2) as f32) * std::f32::consts::PI;
            let offset = ((x_dir * fraction.cos()) + (y_dir * fraction.sin())) * radius;

            let p0 = base_center + offset;
            let p1 = vertex;
            let p2 = base_center - offset;
            self.add_line_strip(vec![p0, p1, p2], color);
        }
    }

    // Returns the draw data, leaving this object in an empty state
    pub fn take_line_lists(&mut self) -> Vec<LineList3D> {
        std::mem::replace(&mut self.line_lists, vec![])
    }

    // Recommended to call every frame to ensure that this doesn't grow unbounded
    pub fn clear(&mut self) {
        self.line_lists.clear();
    }
}


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

    // Buffers for sending data to be rendered to the GPU
    // Indexed by present index, then vertex buffer.
    pub vertex_buffers: Vec<Vec<ManuallyDrop<VkBuffer>>>,
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

        //
        // Containers for dynamically-allocated resources
        //
        let mut vertex_buffers = Vec::with_capacity(swapchain.swapchain_info.image_count);
        for _ in 0..swapchain.swapchain_info.image_count {
            vertex_buffers.push(vec![]);
        }

        Ok(VkDebugRenderPass {
            device_context: device_context.clone(),
            swapchain_info: swapchain.swapchain_info.clone(),
            pipeline_info,
            frame_buffers,
            command_pool,
            command_buffers,
            vertex_buffers,
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
                let framebuffer_attachments =
                    if swapchain_info.msaa_level == MsaaLevel::Sample1 {
                        vec![color_target_image_view, depth_image_view]
                    } else {
                        vec![color_target_image_view, depth_image_view, color_resolved_image_view]
                    };

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
        vertex_buffers: &mut Vec<ManuallyDrop<VkBuffer>>,
        descriptor_set_per_pass: &vk::DescriptorSet,
        line_lists: Vec<LineList3D>,
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

        fn drop_old_buffers(buffers: &mut Vec<ManuallyDrop<VkBuffer>>) {
            for b in buffers.iter_mut() {
                unsafe {
                    ManuallyDrop::drop(b);
                }
            }

            buffers.clear();
        }

        drop_old_buffers(vertex_buffers);

        #[derive(Debug)]
        struct DrawCall {
            first_element: u32,
            count: u32,
        }

        let mut draw_calls = Vec::with_capacity(line_lists.len());

        let mut vertex_list: Vec<DebugVertex> = vec![];
        for line_list in &line_lists {
            let draw_call = DrawCall {
                first_element: 0,
                count: 4,
            };

            let vertex_buffer_first_element = vertex_list.len() as u32;

            for vertex_pos in &line_list.points {
                vertex_list.push(DebugVertex {
                    pos: (*vertex_pos).into(),
                    color: line_list.color.into()
                });
            }

            let draw_call = DrawCall {
                first_element: vertex_buffer_first_element,
                count: line_list.points.len() as u32,
            };

            draw_calls.push(draw_call);
        }

        if !draw_calls.is_empty() {
            let vertex_buffer = {
                let vertex_buffer_size =
                    vertex_list.len() as u64 * std::mem::size_of::<DebugVertex>() as u64;
                let mut vertex_buffer = VkBuffer::new(
                    device_context,
                    vk_mem::MemoryUsage::CpuToGpu,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    vertex_buffer_size,
                )?;

                vertex_buffer.write_to_host_visible_buffer(vertex_list.as_slice())?;
                vertex_buffer
            };

            vertex_buffers.push(ManuallyDrop::new(vertex_buffer));
        }

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

            if !line_lists.is_empty() {
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

                logical_device.cmd_bind_vertex_buffers(
                    *command_buffer,
                    0, // first binding
                    &[vertex_buffers[0].buffer()],
                    &[0], // offsets
                );

                for draw_call in draw_calls {
                    logical_device.cmd_draw(
                        *command_buffer,
                        draw_call.count as u32,
                        1,
                        draw_call.first_element as u32,
                        0,
                    );
                }
            }

            logical_device.cmd_end_render_pass(*command_buffer);
            logical_device.end_command_buffer(*command_buffer)
        }
    }

    pub fn update(
        &mut self,
        present_index: usize,
        descriptor_set_per_pass: vk::DescriptorSet,
        line_lists: Vec<LineList3D>,
    ) -> VkResult<()> {
        Self::update_command_buffer(
            &self.device_context,
            &self.swapchain_info,
            &self.pipeline_info.renderpass.get_raw(),
            self.frame_buffers[present_index],
            &self.pipeline_info.pipeline.get_raw().pipelines[0],
            &self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
            &self.command_buffers[present_index],
            &mut self.vertex_buffers[present_index],
            &descriptor_set_per_pass,
            line_lists,
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

            drop_all_buffer_lists(&mut self.vertex_buffers);

            device.destroy_command_pool(self.command_pool, None);

            for frame_buffer in &self.frame_buffers {
                device.destroy_framebuffer(*frame_buffer, None);
            }
        }

        log::trace!("destroyed VkSpriteRenderPass");
    }
}
