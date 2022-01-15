use rafx::render_feature_write_job_prelude::*;

use super::*;
use rafx::api::{
    RafxIndexBufferBinding, RafxIndexType, RafxPrimitiveTopology, RafxVertexAttributeRate,
    RafxVertexBufferBinding,
};
use rafx::framework::{MaterialPassResource, ResourceArc, VertexDataLayout, VertexDataSetLayout};
use std::marker::PhantomData;

lazy_static::lazy_static! {
    pub static ref IMGUI_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let vertex = imgui::DrawVert {
            pos: Default::default(),
            col: Default::default(),
            uv: Default::default()
        };

        VertexDataLayout::build_vertex_layout(&vertex, RafxVertexAttributeRate::Vertex,  |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.col, "COLOR", RafxFormat::R8G8B8A8_UNORM);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

pub struct ImGuiWriteJob<'write> {
    imgui_material_pass: Option<ResourceArc<MaterialPassResource>>,
    frame_packet: Box<ImGuiFramePacket>,
    submit_packet: Box<ImGuiSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> ImGuiWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<ImGuiFramePacket>,
        submit_packet: Box<ImGuiSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            imgui_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .imgui_material_pass
                    .clone()
            },
            frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for ImGuiWriteJob<'write> {
    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex {
        self.frame_packet.view_frame_index(view)
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

        if per_frame_data.imgui_draw_data.is_some() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    Some(render_phase_index),
                    self.imgui_material_pass.as_ref().unwrap(),
                    &write_context.render_target_meta,
                    &*IMGUI_VERTEX_LAYOUT,
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

            let mut draw_list_index = 0;
            if let Some(draw_data) = &per_frame_data.imgui_draw_data {
                for draw_list in draw_data.draw_lists() {
                    command_buffer.cmd_bind_vertex_buffers(
                        0, // first binding
                        &[RafxVertexBufferBinding {
                            buffer: &per_frame_submit_data.vertex_buffers[draw_list_index]
                                .get_raw()
                                .buffer,
                            byte_offset: 0,
                        }],
                    )?;

                    command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
                        buffer: &per_frame_submit_data.index_buffers[draw_list_index]
                            .get_raw()
                            .buffer,
                        byte_offset: 0,
                        index_type: RafxIndexType::Uint16,
                    })?;

                    let mut element_begin_index: u32 = 0;
                    for cmd in draw_list.commands() {
                        match cmd {
                            ImGuiDrawCmd::Elements {
                                count,
                                cmd_params:
                                    imgui::DrawCmdParams {
                                        clip_rect,
                                        //texture_id,
                                        ..
                                    },
                            } => {
                                let element_end_index = element_begin_index + *count as u32;

                                let scissor_x = ((clip_rect[0] - draw_data.display_pos[0])
                                    * draw_data.framebuffer_scale[0])
                                    as u32;

                                let scissor_y = ((clip_rect[1] - draw_data.display_pos[1])
                                    * draw_data.framebuffer_scale[1])
                                    as u32;

                                let scissor_width =
                                    ((clip_rect[2] - clip_rect[0] - draw_data.display_pos[0])
                                        * draw_data.framebuffer_scale[0])
                                        as u32;

                                let scissor_height =
                                    ((clip_rect[3] - clip_rect[1] - draw_data.display_pos[1])
                                        * draw_data.framebuffer_scale[1])
                                        as u32;

                                command_buffer.cmd_set_scissor(
                                    scissor_x,
                                    scissor_y,
                                    scissor_width,
                                    scissor_height,
                                )?;

                                command_buffer.cmd_draw_indexed(
                                    element_end_index - element_begin_index,
                                    element_begin_index,
                                    0,
                                )?;

                                element_begin_index = element_end_index;
                            }
                            _ => panic!("unexpected draw command"),
                        }
                    }

                    draw_list_index += 1;
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
