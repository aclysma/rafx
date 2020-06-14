use crate::features::sprite::{SpriteRenderFeature, SpriteDrawCall};
use renderer_base::{RenderFeatureIndex, RenderFeature, SubmitNodeId, FeatureCommandWriter};
use crate::RenderJobWriteContext;
use renderer_shell_vulkan::{VkBuffer, VkBufferRaw};
use std::mem::ManuallyDrop;
use crate::resource_managers::ResourceArc;
use ash::vk;
use ash::version::DeviceV1_0;

pub struct SpriteCommandWriter {
    pub vertex_buffers: Vec<ResourceArc<VkBufferRaw>>,
    pub index_buffers: Vec<ResourceArc<VkBufferRaw>>,
    pub draw_calls: Vec<SpriteDrawCall>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for SpriteCommandWriter {
    fn apply_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
    ) {
        //println!("apply");
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        index: SubmitNodeId,
    ) {
        let logical_device = write_context.device_context.device();
        let command_buffer = write_context.command_buffer;

        // //println!("render");
        // logical_device.cmd_bind_pipeline(
        //     *command_buffer,
        //     vk::PipelineBindPoint::GRAPHICS,
        //     *pipeline,
        // );
        //
        // // Bind per-pass data (UBO with view/proj matrix, sampler)
        // logical_device.cmd_bind_descriptor_sets(
        //     *command_buffer,
        //     vk::PipelineBindPoint::GRAPHICS,
        //     *pipeline_layout,
        //     0,
        //     &[*descriptor_set_per_pass],
        //     &[],
        // );
        //
        // logical_device.cmd_bind_vertex_buffers(
        //     *command_buffer,
        //     0, // first binding
        //     &[vertex_buffers[0].buffer()],
        //     &[0], // offsets
        // );
        //
        // logical_device.cmd_bind_index_buffer(
        //     *command_buffer,
        //     index_buffers[0].buffer(),
        //     0, // offset
        //     vk::IndexType::UINT16,
        // );
        //
        // for draw_call in &self.draw_calls {
        //     // Bind per-draw-call data (i.e. texture)
        //     logical_device.cmd_bind_descriptor_sets(
        //         *command_buffer,
        //         vk::PipelineBindPoint::GRAPHICS,
        //         *pipeline_layout,
        //         1,
        //         &[descriptor_set_per_texture[draw_call.texture_descriptor_index as usize]],
        //         &[],
        //     );
        //
        //     logical_device.cmd_draw_indexed(
        //         *command_buffer,
        //         draw_call.index_buffer_count as u32,
        //         1,
        //         draw_call.index_buffer_first_element as u32,
        //         0,
        //         0,
        //     );
        // }
    }

    fn revert_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
    ) {

    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}

// impl Drop for SpriteCommandWriter {
//     fn drop(&mut self) {
//         for buffer in &mut self.vertex_buffers {
//             unsafe {
//                 ManuallyDrop::drop(buffer);
//             }
//         }
//
//         for buffer in &mut self.index_buffers {
//             unsafe {
//                 ManuallyDrop::drop(buffer);
//             }
//         }
//     }
// }