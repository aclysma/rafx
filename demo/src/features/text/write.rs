use rafx::render_feature_write_job_prelude::*;

use rafx::api::RafxFormat;
use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct TextVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

lazy_static::lazy_static! {
    pub static ref TEXT_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataLayout::build_vertex_layout(&TextVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R32G32B32A32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

use super::internal::TextImageUpdate;
use rafx::api::{
    RafxCmdCopyBufferToTextureParams, RafxIndexBufferBinding, RafxIndexType, RafxResourceState,
    RafxTextureBarrier, RafxVertexBufferBinding,
};
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::nodes::{push_view_indexed_value, RenderJobBeginExecuteGraphContext, RenderViewIndex};

#[derive(Debug)]
pub struct TextDrawCallMeta {
    pub font_descriptor_index: u32,
    pub buffer_index: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub z_position: f32,
}

struct TextDrawCallBuffers {
    pub vertex_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
}

pub struct FeatureCommandWriterImpl {
    draw_call_buffers: Vec<TextDrawCallBuffers>,
    draw_call_metas: Vec<TextDrawCallMeta>,
    text_material_pass: ResourceArc<MaterialPassResource>,
    per_font_descriptor_sets: Vec<DescriptorSetArc>,
    per_view_descriptor_sets: Vec<Option<DescriptorSetArc>>,
    image_updates: Vec<TextImageUpdate>,
}

impl FeatureCommandWriterImpl {
    pub fn new(
        text_material_pass: ResourceArc<MaterialPassResource>,
        draw_call_metas: Vec<TextDrawCallMeta>,
        image_updates: Vec<TextImageUpdate>,
        num_draw_call_buffers: usize,
    ) -> Self {
        FeatureCommandWriterImpl {
            draw_call_buffers: Vec::with_capacity(num_draw_call_buffers),
            draw_call_metas,
            text_material_pass,
            per_font_descriptor_sets: Default::default(),
            per_view_descriptor_sets: Default::default(),
            image_updates,
        }
    }

    pub fn push_buffers(
        &mut self,
        vertex_buffer: ResourceArc<BufferResource>,
        index_buffer: ResourceArc<BufferResource>,
    ) {
        self.draw_call_buffers.push(TextDrawCallBuffers {
            vertex_buffer,
            index_buffer,
        });
    }

    pub fn push_per_font_descriptor_set(
        &mut self,
        per_font_descriptor_set: DescriptorSetArc,
    ) {
        self.per_font_descriptor_sets.push(per_font_descriptor_set);
    }

    pub fn push_per_view_descriptor_set(
        &mut self,
        view_index: RenderViewIndex,
        per_view_descriptor_set: DescriptorSetArc,
    ) {
        push_view_indexed_value(
            &mut self.per_view_descriptor_sets,
            view_index,
            per_view_descriptor_set,
        );
    }

    pub fn draw_call_metas(&self) -> &Vec<TextDrawCallMeta> {
        &self.draw_call_metas
    }
}

impl FeatureCommandWriter for FeatureCommandWriterImpl {
    fn on_begin_execute_graph(
        &self,
        write_context: &mut RenderJobBeginExecuteGraphContext,
    ) -> RafxResult<()> {
        profiling::scope!(super::on_begin_execute_graph_scope);

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
        profiling::scope!(super::apply_setup_scope);

        if !self.draw_call_metas.is_empty() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    render_phase_index,
                    &self.text_material_pass,
                    &write_context.render_target_meta,
                    &*TEXT_VERTEX_LAYOUT,
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
        profiling::scope!(super::render_element_scope);

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
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
