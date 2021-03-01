use crate::features::text::{TextImageUpdate, TextRenderFeature};
use rafx::api::{
    RafxCmdCopyBufferToTextureParams, RafxIndexBufferBinding, RafxIndexType, RafxResourceState,
    RafxResult, RafxTextureBarrier, RafxVertexBufferBinding,
};
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::nodes::{
    FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderJobBeginExecuteGraphContext,
    RenderJobWriteContext, RenderPhaseIndex, RenderView, SubmitNodeId,
};

#[derive(Debug)]
pub struct TextDrawCallMeta {
    pub font_descriptor_index: u32,
    pub buffer_index: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub z_position: f32,
}

pub struct TextDrawCallBuffers {
    pub vertex_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
}

pub struct TextCommandWriter {
    pub(super) draw_call_buffers: Vec<TextDrawCallBuffers>,
    pub(super) draw_call_metas: Vec<TextDrawCallMeta>,
    pub(super) text_material_pass: ResourceArc<MaterialPassResource>,
    pub(super) per_font_descriptor_sets: Vec<DescriptorSetArc>,
    pub(super) per_view_descriptor_sets: Vec<Option<DescriptorSetArc>>,
    pub(super) image_updates: Vec<TextImageUpdate>,
}

impl FeatureCommandWriter for TextCommandWriter {
    fn on_begin_execute_graph(
        &self,
        write_context: &mut RenderJobBeginExecuteGraphContext,
    ) -> RafxResult<()> {
        for image_update in &self.image_updates {
            let rafx_image = &image_update.upload_image.get_raw().image.get_raw().image;

            write_context.command_buffer.cmd_resource_barrier(
                &[],
                &[RafxTextureBarrier::state_transition(
                    rafx_image,
                    RafxResourceState::SHADER_RESOURCE,
                    RafxResourceState::COPY_DST,
                )],
            )?;

            log::debug!(
                "upload font atlas data {} bytes",
                image_update
                    .upload_buffer
                    .get_raw()
                    .buffer
                    .buffer_def()
                    .size
            );

            // copy buffer to texture
            write_context.command_buffer.cmd_copy_buffer_to_texture(
                &image_update.upload_buffer.get_raw().buffer,
                rafx_image,
                &RafxCmdCopyBufferToTextureParams::default(),
            )?;

            if rafx_image.texture_def().mip_count > 1 {
                rafx::api::extra::mipmaps::generate_mipmaps(
                    &*write_context.command_buffer,
                    rafx_image,
                )?;
            }

            write_context.command_buffer.cmd_resource_barrier(
                &[],
                &[RafxTextureBarrier::state_transition(
                    rafx_image,
                    RafxResourceState::COPY_DST,
                    RafxResourceState::SHADER_RESOURCE,
                )],
            )?;
        }

        Ok(())
    }

    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        if !self.draw_call_metas.is_empty() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    render_phase_index,
                    &self.text_material_pass,
                    &write_context.render_target_meta,
                    &*super::TEXT_VERTEX_LAYOUT,
                )?;

            let command_buffer = &write_context.command_buffer;
            command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;

            self.per_view_descriptor_sets[view.view_index() as usize]
                .as_ref()
                .unwrap()
                .bind(command_buffer)?;
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
        let draw_call = &self.draw_call_metas[index as usize];
        let buffers = &self.draw_call_buffers[draw_call.buffer_index as usize];
        let command_buffer = &write_context.command_buffer;
        command_buffer.cmd_bind_vertex_buffers(
            0,
            &[RafxVertexBufferBinding {
                buffer: &buffers.vertex_buffer.get_raw().buffer,
                byte_offset: 0,
            }],
        )?;

        command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
            buffer: &buffers.index_buffer.get_raw().buffer,
            index_type: RafxIndexType::Uint16,
            byte_offset: 0,
        })?;

        self.per_font_descriptor_sets[draw_call.font_descriptor_index as usize]
            .bind(command_buffer)?;

        command_buffer.cmd_draw_indexed(draw_call.index_count, draw_call.index_offset, 0)?;
        Ok(())
    }

    fn feature_debug_name(&self) -> &'static str {
        TextRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        TextRenderFeature::feature_index()
    }
}
