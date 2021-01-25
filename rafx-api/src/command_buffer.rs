#[cfg(feature = "rafx-metal")]
use crate::metal::RafxCommandBufferMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxCommandBufferVulkan;
use crate::{
    RafxBuffer, RafxBufferBarrier, RafxCmdBlitParams, RafxCmdCopyBufferToTextureParams,
    RafxColorRenderTargetBinding, RafxDepthRenderTargetBinding, RafxDescriptorSetArray,
    RafxDescriptorSetHandle, RafxIndexBufferBinding, RafxPipeline, RafxRenderTargetBarrier,
    RafxResult, RafxRootSignature, RafxTexture, RafxTextureBarrier, RafxVertexBufferBinding,
};

/// A command buffer contains a list of work for the GPU to do.
///
/// It cannot be created directly. It must be allocated out of a pool.
///
/// The command pool and all command buffers allocated from it share memory. The standard rust rules
/// about mutability apply but are not enforced at compile time or runtime.
///  * Do not modify two command buffers from the same pool concurrently
///  * Do not allocate from a command pool while modifying one of its command buffers
///  * Once a command buffer is submitted to the GPU, do not modify its pool, or any command buffers
///    created from it, until the GPU completes its work.
///
/// In general, do not modify textures, buffers, command buffers, or other GPU resources while a
/// command buffer referencing them is submitted. Additionally, these resources must persist for
/// the entire duration of the submitted workload.
///
/// Semaphores and fences can be used for achieve the more fine-grained scheduling necessary to
/// modify resources that are referenced from a submitted and in-use command buffer.
///
/// Command pools MAY be dropped if they are in use by the GPU, but the command pool must not be
/// dropped. Dropped command pools that are not returned to the pool will not be available for
/// reuse.
#[derive(Debug)]
pub enum RafxCommandBuffer {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxCommandBufferVulkan),

    #[cfg(feature = "rafx-metal")]
    Metal(RafxCommandBufferMetal),
}

impl RafxCommandBuffer {
    /// Begins writing a command buffer. This can only be called when the command buffer is first
    /// allocated or if the pool has been reset since it was last written
    pub fn begin(&self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.begin(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.begin(),
        }
    }

    /// End writing the command buffer. This must be called before submitting the command buffer
    /// to the GPU
    pub fn end(&self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.end(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.end(),
        }
    }

    /// This returns the command buffer to the pool, allowing it to be allocated again. This must
    /// not be called if the command buffer is still in-use by the GPU.
    ///
    /// Dropping a command buffer without returning it to the pool is allowed. In this case, it
    /// remains usable by the GPU until the command pool is dropped. However, even if the command
    /// buffer is reset, this command buffer will not be available for use again.
    pub fn return_to_pool(&self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.return_to_pool(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.return_to_pool(),
        }
    }

    /// Begin a new renderpass using the given color targets and depth targets. This is similar to
    /// beginning a renderpass in vulkan.
    ///
    /// Some command must be used within a renderpass and some may only be used outside of a
    /// renderpass.
    pub fn cmd_bind_render_targets(
        &self,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<RafxDepthRenderTargetBinding>,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_bind_render_targets(color_targets, depth_target)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => {
                inner.cmd_bind_render_targets(color_targets, depth_target)
            }
        }
    }

    /// Finish the renderpass.
    pub fn cmd_unbind_render_targets(&self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_unbind_render_targets(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_unbind_render_targets(),
        }
    }

    /// Set the viewport state. This may be called inside or outside of a renderpass.
    ///
    /// Viewport state defines where on the screen the draw will occur.
    pub fn cmd_set_viewport(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        depth_min: f32,
        depth_max: f32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_set_viewport(x, y, width, height, depth_min, depth_max)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => {
                inner.cmd_set_viewport(x, y, width, height, depth_min, depth_max)
            }
        }
    }

    /// Set the scissor state. This may be called inside or outside of a renderpass.
    ///
    /// Scissor state can be used to restrict rendering to a specific area of a render target
    pub fn cmd_set_scissor(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_set_scissor(x, y, width, height),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_set_scissor(x, y, width, height),
        }
    }

    /// Set the stencil buffer state. This may be called inside or outside of a renderpass.
    ///
    /// Stencil buffer state is used with a stencil render target to discard rendering results in
    /// specific portions of a render target
    pub fn cmd_set_stencil_reference_value(
        &self,
        value: u32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_set_stencil_reference_value(value),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_set_stencil_reference_value(value),
        }
    }

    /// Binds the given pipeline - which represents fixed-function state and shaders. Draw calls
    /// that produce primitives or dispatch compute will use the bound pipeline.
    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipeline,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_bind_pipeline(pipeline.vk_pipeline().unwrap())
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => {
                inner.cmd_bind_pipeline(pipeline.metal_pipeline().unwrap())
            }
        }
    }

    /// Binds a buffer as a vertex buffer. Draw calls will use this buffer as input.
    ///
    /// Multiple buffers can be bound, but the number is limited depending on API/hardware. Less
    /// than 4 is a relatively safe number.
    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[RafxVertexBufferBinding],
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_vertex_buffers(first_binding, bindings),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => {
                inner.cmd_bind_vertex_buffers(first_binding, bindings)
            }
        }
    }

    /// Binds a buffer as a vertex buffer. Draw calls will use this buffer as input.
    ///
    /// Multiple buffers can be bound, but the number is limited depending on API/hardware. Less
    /// than 4 is a relatively safe number.
    pub fn cmd_bind_index_buffer(
        &self,
        binding: &RafxIndexBufferBinding,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_index_buffer(binding),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_bind_index_buffer(binding),
        }
    }

    /// Binds a descriptor set for use by the shader in the currently bound pipeline.
    ///
    /// Multiple descriptor sets can be bound, but the number is limited to 4.
    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArray,
        index: u32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_descriptor_set(
                descriptor_set_array.vk_descriptor_set_array().unwrap(),
                index,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_bind_descriptor_set(
                descriptor_set_array.metal_descriptor_set_array().unwrap(),
                index,
            ),
        }
    }

    /// Binds a descriptor set for use by the shader in the currently bound pipeline.
    ///
    /// This is the same as `cmd_bind_descriptor_set` but uses a lightweight, opaque handle. This
    /// may make using the API easier in multi-threaded scenarios.
    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignature,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandle,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_descriptor_set_handle(
                root_signature.vk_root_signature().unwrap(),
                set_index,
                descriptor_set_handle.vk_descriptor_set_handle().unwrap(),
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_bind_descriptor_set_handle(
                root_signature.metal_root_signature().unwrap(),
                set_index,
                descriptor_set_handle.metal_descriptor_set_handle().unwrap(),
            ),
        }
    }

    /// Draw primitives using the currently bound pipeline and vertex buffer
    pub fn cmd_draw(
        &self,
        vertex_count: u32,
        first_vertex: u32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_draw(vertex_count, first_vertex),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_draw(vertex_count, first_vertex),
        }
    }

    /// Draw instanced primitives using the currently bound pipeline and vertex buffer
    pub fn cmd_draw_instanced(
        &self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        first_instance: u32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_draw_instanced(vertex_count, first_vertex, instance_count, first_instance)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => {
                inner.cmd_draw_instanced(vertex_count, first_vertex, instance_count, first_instance)
            }
        }
    }

    /// Draw primitives using the currently bound pipeline, vertex, and index buffer
    pub fn cmd_draw_indexed(
        &self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_draw_indexed(index_count, first_index, vertex_offset)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => {
                inner.cmd_draw_indexed(index_count, first_index, vertex_offset)
            }
        }
    }

    /// Draw instanced primitives using the currently bound pipeline, vertex, and index buffer
    pub fn cmd_draw_indexed_instanced(
        &self,
        index_count: u32,
        first_index: u32,
        instance_count: u32,
        first_instance: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_draw_indexed_instanced(
                index_count,
                first_index,
                instance_count,
                first_instance,
                vertex_offset,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_draw_indexed_instanced(
                index_count,
                first_index,
                instance_count,
                first_instance,
                vertex_offset,
            ),
        }
    }

    /// Dispatch the current pipeline. Only usable with compute pipelines.
    pub fn cmd_dispatch(
        &self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_dispatch(group_count_x, group_count_y, group_count_z)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => {
                inner.cmd_dispatch(group_count_x, group_count_y, group_count_z)
            }
        }
    }

    /// Add a memory barrier for one or more resources. This must occur OUTSIDE of a renderpass.
    pub fn cmd_resource_barrier(
        &self,
        buffer_barriers: &[RafxBufferBarrier],
        texture_barriers: &[RafxTextureBarrier],
        render_target_barriers: &[RafxRenderTargetBarrier],
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_resource_barrier(
                buffer_barriers,
                texture_barriers,
                render_target_barriers,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_resource_barrier(
                buffer_barriers,
                texture_barriers,
                render_target_barriers,
            ),
        }
    }

    /// Copy the contents of one buffer into another. This occurs on the GPU and allows modifying
    /// resources that are not accessible to the CPU.
    pub fn cmd_copy_buffer_to_buffer(
        &self,
        src_buffer: &RafxBuffer,
        dst_buffer: &RafxBuffer,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_copy_buffer_to_buffer(
                src_buffer.vk_buffer().unwrap(),
                dst_buffer.vk_buffer().unwrap(),
                src_offset,
                dst_offset,
                size,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_copy_buffer_to_buffer(
                src_buffer.metal_buffer().unwrap(),
                dst_buffer.metal_buffer().unwrap(),
                src_offset,
                dst_offset,
                size,
            ),
        }
    }

    /// Copy the contents of a buffer into a texture. This occurs on the GPU and allows modifying
    /// resources that are not accessible to the CPU.
    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBuffer,
        dst_texture: &RafxTexture,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_copy_buffer_to_texture(
                src_buffer.vk_buffer().unwrap(),
                dst_texture.vk_texture().unwrap(),
                params,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_copy_buffer_to_texture(
                src_buffer.metal_buffer().unwrap(),
                dst_texture.metal_texture().unwrap(),
                params,
            ),
        }
    }

    /// Copy a portion of one texture into another texture. This occurs on the GPU and allows
    /// modifying resources that are not accessible to the CPU.
    pub fn cmd_blit(
        &self,
        src_texture: &RafxTexture,
        dst_texture: &RafxTexture,
        params: &RafxCmdBlitParams,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => inner.cmd_blit(
                src_texture.vk_texture().unwrap(),
                dst_texture.vk_texture().unwrap(),
                params,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.cmd_blit(
                src_texture.metal_texture().unwrap(),
                dst_texture.metal_texture().unwrap(),
                params,
            ),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_command_buffer(&self) -> Option<&RafxCommandBufferVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_command_buffer(&self) -> Option<&RafxCommandBufferMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxCommandBuffer::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => Some(inner),
        }
    }
}
