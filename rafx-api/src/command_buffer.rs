#[cfg(feature = "rafx-metal")]
use crate::metal::RafxCommandBufferMetal;
use crate::vulkan::RafxCommandBufferVulkan;
use crate::{
    RafxBuffer, RafxBufferBarrier, RafxCmdBlitParams, RafxCmdCopyBufferToTextureParams,
    RafxColorRenderTargetBinding, RafxDepthRenderTargetBinding, RafxDescriptorSetArray,
    RafxDescriptorSetHandle, RafxIndexBufferBinding, RafxPipeline, RafxRenderTargetBarrier,
    RafxResult, RafxRootSignature, RafxTexture, RafxTextureBarrier, RafxVertexBufferBinding,
};

#[derive(Debug)]
pub enum RafxCommandBuffer {
    Vk(RafxCommandBufferVulkan),

    #[cfg(feature = "rafx-metal")]
    Metal(RafxCommandBufferMetal),
}

impl RafxCommandBuffer {
    pub fn begin(&self) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.begin(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.begin(),
        }
    }

    pub fn end(&self) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.end(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(inner) => inner.end(),
        }
    }

    pub fn return_to_pool(&self) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.return_to_pool(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn cmd_bind_render_targets(
        &self,
        color_targets: &[RafxColorRenderTargetBinding],
        depth_target: Option<RafxDepthRenderTargetBinding>,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_bind_render_targets(color_targets, depth_target)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_unbind_render_targets(&self) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_unbind_render_targets(),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

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
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_set_viewport(x, y, width, height, depth_min, depth_max)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_set_scissor(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_set_scissor(x, y, width, height),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_set_stencil_reference_value(
        &self,
        value: u32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_set_stencil_reference_value(value),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_bind_pipeline(
        &self,
        pipeline: &RafxPipeline,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_bind_pipeline(pipeline.vk_pipeline().unwrap())
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_bind_vertex_buffers(
        &self,
        first_binding: u32,
        bindings: &[RafxVertexBufferBinding],
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_vertex_buffers(first_binding, bindings),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_bind_descriptor_set(
        &self,
        descriptor_set_array: &RafxDescriptorSetArray,
        index: u32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_descriptor_set(
                descriptor_set_array.vk_descriptor_set_array().unwrap(),
                index,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_bind_descriptor_set_handle(
        &self,
        root_signature: &RafxRootSignature,
        set_index: u32,
        descriptor_set_handle: &RafxDescriptorSetHandle,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_descriptor_set_handle(
                root_signature.vk_root_signature().unwrap(),
                set_index,
                descriptor_set_handle.vk_descriptor_set_handle().unwrap(),
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_bind_index_buffer(
        &self,
        binding: &RafxIndexBufferBinding,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_bind_index_buffer(binding),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_draw(
        &self,
        vertex_count: u32,
        first_vertex: u32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_draw(vertex_count, first_vertex),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_draw_instanced(
        &self,
        vertex_count: u32,
        first_vertex: u32,
        instance_count: u32,
        first_instance: u32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_draw_instanced(vertex_count, first_vertex, instance_count, first_instance)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_draw_indexed(
        &self,
        index_count: u32,
        first_index: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_draw_indexed(index_count, first_index, vertex_offset)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_draw_indexed_instanced(
        &self,
        index_count: u32,
        first_index: u32,
        instance_count: u32,
        first_instance: u32,
        vertex_offset: i32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_draw_indexed_instanced(
                index_count,
                first_index,
                instance_count,
                first_instance,
                vertex_offset,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_dispatch(
        &self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => {
                inner.cmd_dispatch(group_count_x, group_count_y, group_count_z)
            }
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_resource_barrier(
        &self,
        buffer_barriers: &[RafxBufferBarrier],
        texture_barriers: &[RafxTextureBarrier],
        render_target_barriers: &[RafxRenderTargetBarrier],
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_resource_barrier(
                buffer_barriers,
                texture_barriers,
                render_target_barriers,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_copy_buffer_to_buffer(
        &self,
        src_buffer: &RafxBuffer,
        dst_buffer: &RafxBuffer,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_copy_buffer_to_buffer(
                src_buffer.vk_buffer().unwrap(),
                dst_buffer.vk_buffer().unwrap(),
                src_offset,
                dst_offset,
                size,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_copy_buffer_to_texture(
        &self,
        src_buffer: &RafxBuffer,
        dst_texture: &RafxTexture,
        params: &RafxCmdCopyBufferToTextureParams,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_copy_buffer_to_texture(
                src_buffer.vk_buffer().unwrap(),
                dst_texture.vk_texture().unwrap(),
                params,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn cmd_blit(
        &self,
        src_texture: &RafxTexture,
        dst_texture: &RafxTexture,
        params: &RafxCmdBlitParams,
    ) -> RafxResult<()> {
        match self {
            RafxCommandBuffer::Vk(inner) => inner.cmd_blit(
                src_texture.vk_texture().unwrap(),
                dst_texture.vk_texture().unwrap(),
                params,
            ),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_inner) => {
                unimplemented!();
            }
        }
    }

    pub fn vk_command_buffer(&self) -> Option<&RafxCommandBufferVulkan> {
        match self {
            RafxCommandBuffer::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxCommandBuffer::Metal(_) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_command_buffer(&self) -> Option<&RafxCommandBufferMetal> {
        match self {
            RafxCommandBuffer::Vk(_) => None,
            RafxCommandBuffer::Metal(inner) => Some(inner),
        }
    }
}
