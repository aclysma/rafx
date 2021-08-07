use rafx::render_feature_write_job_prelude::*;

use super::*;
use rafx::api::{
    RafxBarrierQueueTransition, RafxCmdCopyBufferToTextureParams, RafxIndexBufferBinding,
    RafxIndexType, RafxPrimitiveTopology, RafxResourceState, RafxTextureBarrier,
    RafxVertexAttributeRate, RafxVertexBufferBinding,
};
use rafx::framework::{MaterialPassResource, ResourceArc, VertexDataLayout, VertexDataSetLayout};
use std::marker::PhantomData;

lazy_static::lazy_static! {
    pub static ref EGUI_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let vertex = egui::epaint::Vertex {
            pos: Default::default(),
            color: Default::default(),
            uv: Default::default()
        };

        VertexDataLayout::build_vertex_layout(&vertex, RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R8G8B8A8_UNORM);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

pub struct EguiWriteJob<'write> {
    egui_material_pass: Option<ResourceArc<MaterialPassResource>>,
    frame_packet: Box<EguiFramePacket>,
    submit_packet: Box<EguiSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> EguiWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<EguiFramePacket>,
        submit_packet: Box<EguiSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            egui_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .egui_material_pass
                    .clone()
            },
            frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for EguiWriteJob<'write> {
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
        if let Some(image_update) = &self
            .submit_packet
            .per_frame_submit_data()
            .get()
            .image_update
        {
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
                "upload egui texture {} bytes",
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
        _view_frame_index: ViewFrameIndex,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().apply_setup);

        let per_frame_data = self.frame_packet.per_frame_data().get();
        let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();

        if per_frame_data.egui_draw_data.is_some() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    render_phase_index,
                    self.egui_material_pass.as_ref().unwrap(),
                    &write_context.render_target_meta,
                    &*EGUI_VERTEX_LAYOUT,
                )?;

            let command_buffer = &write_context.command_buffer;
            command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

            per_frame_submit_data
                .per_view_descriptor_set
                .as_ref()
                .unwrap()
                .bind(command_buffer)?; // view/projection

            per_frame_submit_data
                .per_font_descriptor_set
                .as_ref()
                .unwrap()
                .bind(command_buffer)?; // font atlas
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

        // The prepare phase emits a single node which will draw everything. In the future it might
        // emit a node per draw call that uses transparency
        if submit_node_id == 0 {
            let per_frame_data = self.frame_packet.per_frame_data().get();
            let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();

            let command_buffer = &write_context.command_buffer;

            if let Some(draw_data) = &per_frame_data.egui_draw_data {
                command_buffer.cmd_bind_vertex_buffers(
                    0, // first binding
                    &[RafxVertexBufferBinding {
                        buffer: &per_frame_submit_data
                            .vertex_buffer
                            .as_ref()
                            .unwrap()
                            .get_raw()
                            .buffer,
                        byte_offset: 0,
                    }],
                )?;

                command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
                    buffer: &per_frame_submit_data
                        .index_buffer
                        .as_ref()
                        .unwrap()
                        .get_raw()
                        .buffer,
                    byte_offset: 0,
                    index_type: RafxIndexType::Uint16,
                })?;

                for clipped_draw_call in &draw_data.clipped_draw_calls {
                    command_buffer.cmd_set_scissor(
                        clipped_draw_call.clip_rect.left() as u32,
                        clipped_draw_call.clip_rect.top() as u32,
                        clipped_draw_call.clip_rect.width() as u32,
                        clipped_draw_call.clip_rect.height() as u32,
                    )?;

                    for draw_call in &clipped_draw_call.draw_calls {
                        command_buffer.cmd_draw_indexed(
                            draw_call.index_count as u32,
                            draw_call.index_offset as u32,
                            draw_call.vertex_offset as i32,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
