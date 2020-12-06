use crate::features::debug3d::{Debug3dDrawCall, Debug3dRenderFeature};
use crate::render_contexts::RenderJobWriteContext;
use ash::version::DeviceV1_0;
use ash::vk;
use rafx::nodes::{
    FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderPhaseIndex, RenderView,
    SubmitNodeId,
};
use rafx::resources::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct Debug3dCommandWriter {
    pub(super) vertex_buffer: Option<ResourceArc<BufferResource>>,
    pub(super) draw_calls: Vec<Debug3dDrawCall>,
    pub(super) debug3d_material_pass: ResourceArc<MaterialPassResource>,
    pub(super) per_view_descriptor_sets: Vec<Option<DescriptorSetArc>>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for Debug3dCommandWriter {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
        if let Some(vertex_buffer) = self.vertex_buffer.as_ref() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    &self.debug3d_material_pass,
                    &write_context.renderpass,
                    &write_context.framebuffer_meta,
                    &*super::DEBUG_VERTEX_LAYOUT,
                )
                .unwrap();

            let logical_device = write_context.device_context.device();
            let command_buffer = write_context.command_buffer;
            unsafe {
                logical_device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipelines[0],
                );

                // Bind per-pass data (UBO with view/proj matrix, sampler)
                logical_device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                    shaders::debug_vert::PER_FRAME_DATA_DESCRIPTOR_SET_INDEX as u32,
                    &[self.per_view_descriptor_sets[view.view_index() as usize]
                        .as_ref()
                        .unwrap()
                        .get()],
                    &[],
                );

                logical_device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0, // first binding
                    &[vertex_buffer.get_raw().buffer.buffer],
                    &[0], // offsets
                );
            }
        }
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
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
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
    }

    fn feature_debug_name(&self) -> &'static str {
        Debug3dRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        Debug3dRenderFeature::feature_index()
    }
}
