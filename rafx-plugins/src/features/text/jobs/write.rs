use rafx::render_feature_write_job_prelude::*;

use super::*;
use rafx::api::{
    RafxBarrierQueueTransition, RafxCmdCopyBufferToTextureParams, RafxFormat,
    RafxIndexBufferBinding, RafxIndexType, RafxPrimitiveTopology, RafxResourceState,
    RafxTextureBarrier, RafxVertexAttributeRate, RafxVertexBufferBinding,
};
use rafx::framework::{MaterialPassResource, ResourceArc, VertexDataLayout, VertexDataSetLayout};
use std::marker::PhantomData;

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
        VertexDataLayout::build_vertex_layout(&TextVertex::default(), RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R32G32B32A32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

pub struct TextWriteJob<'write> {
    text_material_pass: Option<ResourceArc<MaterialPassResource>>,
    frame_packet: Box<TextFramePacket>,
    submit_packet: Box<TextSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> TextWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<TextFramePacket>,
        submit_packet: Box<TextSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            text_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .text_material_pass
                    .clone()
            },
            frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for TextWriteJob<'write> {
    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex {
        self.frame_packet.view_frame_index(view)
    }

    fn on_begin_execute_graph(
        &self,
        begin_execute_graph_context: &mut RenderJobBeginExecuteGraphContext,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().on_begin_execute_graph);
        let image_updates = &self
            .submit_packet
            .per_frame_submit_data()
            .get()
            .image_updates;

        for image_update in image_updates.iter() {
            let rafx_image = &image_update.upload_image.get_raw().image.get_raw().image;

            begin_execute_graph_context
                .command_buffer
                .cmd_resource_barrier(
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
            begin_execute_graph_context
                .command_buffer
                .cmd_copy_buffer_to_texture(
                    &image_update.upload_buffer.get_raw().buffer,
                    rafx_image,
                    &RafxCmdCopyBufferToTextureParams::default(),
                )?;

            if rafx_image.texture_def().mip_count > 1 {
                begin_execute_graph_context
                    .command_buffer
                    .cmd_resource_barrier(
                        &[],
                        &[RafxTextureBarrier {
                            texture: &rafx_image,
                            src_state: RafxResourceState::COPY_DST,
                            dst_state: RafxResourceState::COPY_SRC,
                            queue_transition: RafxBarrierQueueTransition::None,
                            array_slice: None,
                            mip_slice: Some(0),
                        }],
                    )?;
                rafx::api::extra::mipmaps::generate_mipmaps(
                    &*begin_execute_graph_context.command_buffer,
                    rafx_image,
                )?;
                begin_execute_graph_context
                    .command_buffer
                    .cmd_resource_barrier(
                        &[],
                        &[RafxTextureBarrier::state_transition(
                            rafx_image,
                            RafxResourceState::COPY_SRC,
                            RafxResourceState::SHADER_RESOURCE,
                        )],
                    )?;
            } else {
                begin_execute_graph_context
                    .command_buffer
                    .cmd_resource_barrier(
                        &[],
                        &[RafxTextureBarrier::state_transition(
                            rafx_image,
                            RafxResourceState::COPY_DST,
                            RafxResourceState::SHADER_RESOURCE,
                        )],
                    )?;
            }
        }

        Ok(())
    }

    fn apply_setup(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        view_frame_index: ViewFrameIndex,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().apply_setup);

        if let Some(text_material_pass) = &self.text_material_pass {
            let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();
            if !per_frame_submit_data.draw_call_metas.is_empty() {
                let pipeline = write_context
                    .resource_context
                    .graphics_pipeline_cache()
                    .get_or_create_graphics_pipeline(
                        Some(render_phase_index),
                        &text_material_pass,
                        &write_context.render_target_meta,
                        &*TEXT_VERTEX_LAYOUT,
                    )?;

                let command_buffer = &write_context.command_buffer;
                command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;

                let view_submit_packet = self.submit_packet.view_submit_packet(view_frame_index);
                view_submit_packet
                    .per_view_submit_data()
                    .get()
                    .descriptor_set_arc
                    .as_ref()
                    .unwrap()
                    .bind(command_buffer)?;
            }
        }

        Ok(())
    }

    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        _view_frame_index: ViewFrameIndex,
        _render_phase_index: RenderPhaseIndex,
        submit_node_id: SubmitNodeId,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();

        let draw_call = &per_frame_submit_data.draw_call_metas[submit_node_id as usize];
        let buffers = &per_frame_submit_data.draw_call_buffers[draw_call.buffer_index as usize];

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

        per_frame_submit_data.per_font_descriptor_sets[draw_call.font_descriptor_index as usize]
            .bind(command_buffer)?;

        command_buffer.cmd_draw_indexed(draw_call.index_count, draw_call.index_offset, 0)?;

        Ok(())
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
