use rafx::render_feature_write_job_prelude::*;

use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};

lazy_static::lazy_static! {
    pub static ref IMGUI_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let vertex = imgui::DrawVert {
            pos: Default::default(),
            col: Default::default(),
            uv: Default::default()
        };

        VertexDataLayout::build_vertex_layout(&vertex, |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.col, "COLOR", RafxFormat::R8G8B8A8_UNORM);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

use super::internal::{ImGuiDrawCmd, ImGuiDrawData};
use rafx::api::{RafxIndexBufferBinding, RafxIndexType, RafxVertexBufferBinding};
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct WriteJobImpl {
    vertex_buffers: Vec<ResourceArc<BufferResource>>,
    index_buffers: Vec<ResourceArc<BufferResource>>,
    per_view_descriptor_set: DescriptorSetArc,
    per_font_descriptor_set: DescriptorSetArc,
    imgui_material_pass: ResourceArc<MaterialPassResource>,
    imgui_draw_data: Option<ImGuiDrawData>,
}

impl WriteJobImpl {
    pub fn new(
        imgui_material_pass: ResourceArc<MaterialPassResource>,
        per_view_descriptor_set: DescriptorSetArc,
        per_font_descriptor_set: DescriptorSetArc,
        num_draw_lists: usize,
    ) -> Self {
        WriteJobImpl {
            vertex_buffers: Vec::with_capacity(num_draw_lists),
            index_buffers: Vec::with_capacity(num_draw_lists),
            per_view_descriptor_set,
            per_font_descriptor_set,
            imgui_material_pass,
            imgui_draw_data: Default::default(),
        }
    }

    pub fn push_buffers(
        &mut self,
        vertex_buffer: ResourceArc<BufferResource>,
        index_buffer: ResourceArc<BufferResource>,
    ) {
        self.vertex_buffers.push(vertex_buffer);
        self.index_buffers.push(index_buffer);
    }

    pub fn set_imgui_draw_data(
        &mut self,
        imgui_draw_data: Option<ImGuiDrawData>,
    ) {
        self.imgui_draw_data = imgui_draw_data;
    }
}

impl FeatureCommandWriter for WriteJobImpl {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        profiling::scope!(super::apply_setup_scope);

        if self.imgui_draw_data.is_some() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    render_phase_index,
                    &self.imgui_material_pass,
                    &write_context.render_target_meta,
                    &*IMGUI_VERTEX_LAYOUT,
                )?;

            let command_buffer = &write_context.command_buffer;
            command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

            self.per_view_descriptor_set.bind(command_buffer)?; // view/projection
            self.per_font_descriptor_set.bind(command_buffer)?; // font atlas
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

        // The prepare phase emits a single node which will draw everything. In the future it might
        // emit a node per draw call that uses transparency
        if index == 0 {
            let command_buffer = &write_context.command_buffer;

            let mut draw_list_index = 0;
            if let Some(draw_data) = &self.imgui_draw_data {
                for draw_list in draw_data.draw_lists() {
                    command_buffer.cmd_bind_vertex_buffers(
                        0, // first binding
                        &[RafxVertexBufferBinding {
                            buffer: &self.vertex_buffers[draw_list_index].get_raw().buffer,
                            byte_offset: 0,
                        }],
                    )?;

                    command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
                        buffer: &self.index_buffers[draw_list_index].get_raw().buffer,
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

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
