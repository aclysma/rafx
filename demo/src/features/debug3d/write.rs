use crate::features::debug3d::{Debug3dRenderFeature, Debug3dDrawCall};
use renderer::nodes::{
    RenderFeatureIndex, RenderFeature, SubmitNodeId, FeatureCommandWriter, RenderView,
};
use crate::render_contexts::RenderJobWriteContext;
use renderer::vulkan::VkBufferRaw;
use renderer::resources::resource_managers::{ResourceArc, PipelineSwapchainInfo, DescriptorSetArc};
use ash::vk;
use ash::version::DeviceV1_0;

pub struct Debug3dCommandWriter {
    pub(super) vertex_buffer: Option<ResourceArc<VkBufferRaw>>,
    pub(super) draw_calls: Vec<Debug3dDrawCall>,
    pub(super) pipeline_info: PipelineSwapchainInfo,
    pub(super) descriptor_set_per_view: Vec<DescriptorSetArc>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for Debug3dCommandWriter {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
    ) {
        if let Some(vertex_buffer) = self.vertex_buffer.as_ref() {
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
                    &[vertex_buffer.get_raw().buffer],
                    &[0], // offsets
                );
            }
        }
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
        index: SubmitNodeId,
    ) {
        // The prepare phase emits a single node which will draw everything. In the future it might
        // emit a node per draw call that uses transparency
        if index == 0 {
            // //println!("render");
            let logical_device = write_context.device_context.device();
            let command_buffer = write_context.command_buffer;

            unsafe {
                for draw_call in &self.draw_calls {
                    logical_device.cmd_draw(
                        command_buffer,
                        draw_call.count as u32,
                        1,
                        draw_call.first_element as u32,
                        0,
                    );
                }
            }
        }
    }

    fn revert_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
        view: &RenderView,
    ) {
    }

    fn feature_debug_name(&self) -> &'static str {
        Debug3dRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        Debug3dRenderFeature::feature_index()
    }
}
