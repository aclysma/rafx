use crate::features::debug3d::{Debug3dDrawCall, Debug3dRenderFeature};
use crate::render_contexts::RenderJobWriteContext;
use rafx::api::{RafxResult, RafxVertexBufferBinding};
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
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        if let Some(vertex_buffer) = self.vertex_buffer.as_ref() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    render_phase_index,
                    &self.debug3d_material_pass,
                    &write_context.render_target_meta,
                    &*super::DEBUG_VERTEX_LAYOUT,
                )?;

            let command_buffer = &write_context.command_buffer;
            command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;

            self.per_view_descriptor_sets[view.view_index() as usize]
                .as_ref()
                .unwrap()
                .bind(command_buffer)?;

            command_buffer.cmd_bind_vertex_buffers(
                0,
                &[RafxVertexBufferBinding {
                    buffer: &*vertex_buffer.get_raw().buffer,
                    offset: 0,
                }],
            )?;
        }
        Ok(())
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> RafxResult<()> {
        // The prepare phase emits a single node which will draw everything. In the future it might
        // emit a node per draw call that uses transparency
        if index == 0 {
            let command_buffer = &write_context.command_buffer;

            for draw_call in &self.draw_calls {
                command_buffer.cmd_draw(draw_call.count as u32, draw_call.first_element as u32)?;
            }
        }
        Ok(())
    }

    fn revert_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn feature_debug_name(&self) -> &'static str {
        Debug3dRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        Debug3dRenderFeature::feature_index()
    }
}
