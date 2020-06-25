use crate::features::sprite::{SpriteRenderFeature, SpriteDrawCall};
use renderer::nodes::{
    RenderFeatureIndex, RenderFeature, SubmitNodeId, FeatureCommandWriter, RenderView,
};
use crate::render_contexts::RenderJobWriteContext;
use renderer::vulkan::VkBufferRaw;
use renderer::resources::resource_managers::{ResourceArc, PipelineSwapchainInfo, DescriptorSetArc};
use ash::vk;
use ash::version::DeviceV1_0;

pub struct SpriteCommandWriter {
    pub vertex_buffers: Vec<ResourceArc<VkBufferRaw>>,
    pub index_buffers: Vec<ResourceArc<VkBufferRaw>>,
    pub draw_calls: Vec<SpriteDrawCall>,
    pub pipeline_info: PipelineSwapchainInfo,
    pub descriptor_set_per_view: Vec<DescriptorSetArc>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for SpriteCommandWriter {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
    ) {
        // println!("render");
        let logical_device = write_context.device_context.device();
        let command_buffer = write_context.command_buffer;
        unsafe {
            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline.get_raw().pipelines[0],
            );

            // Bind per-pass data (UBO with view/proj matrix, sampler)
            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
                0,
                &[self.descriptor_set_per_view[view.view_index() as usize].get()],
                &[],
            );

            logical_device.cmd_bind_vertex_buffers(
                command_buffer,
                0, // first binding
                &[self.vertex_buffers[0].get_raw().buffer],
                &[0], // offsets
            );

            logical_device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffers[0].get_raw().buffer,
                0, // offset
                vk::IndexType::UINT16,
            );
        }
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        index: SubmitNodeId,
    ) {
        // //println!("render");
        let logical_device = write_context.device_context.device();
        let command_buffer = write_context.command_buffer;
        let draw_call = &self.draw_calls[index as usize];

        unsafe {
            // Bind per-draw-call data (i.e. texture)
            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
                1,
                &[draw_call.texture_descriptor_set.get()],
                &[],
            );

            logical_device.cmd_draw_indexed(
                command_buffer,
                draw_call.index_buffer_count as u32,
                1,
                draw_call.index_buffer_first_element as u32,
                0,
                0,
            );

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
    }

    fn revert_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
    ) {
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}
