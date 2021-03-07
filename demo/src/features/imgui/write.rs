use super::imgui_draw_data::{ImGuiDrawCmd, ImGuiDrawData};
use crate::features::imgui::ImGuiRenderFeature;
use rafx::api::{RafxIndexBufferBinding, RafxIndexType, RafxResult, RafxVertexBufferBinding};
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::nodes::{
    FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderJobWriteContext,
    RenderPhaseIndex, RenderView, SubmitNodeId,
};

pub struct ImGuiCommandWriter {
    pub(super) vertex_buffers: Vec<ResourceArc<BufferResource>>,
    pub(super) index_buffers: Vec<ResourceArc<BufferResource>>,
    pub(super) imgui_draw_data: Option<ImGuiDrawData>,
    pub(super) per_pass_descriptor_set: DescriptorSetArc,
    pub(super) per_image_descriptor_sets: Vec<DescriptorSetArc>,
    pub(super) imgui_material_pass: ResourceArc<MaterialPassResource>,
}

impl FeatureCommandWriter for ImGuiCommandWriter {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        if self.imgui_draw_data.is_some() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    render_phase_index,
                    &self.imgui_material_pass,
                    &write_context.render_target_meta,
                    &*super::IMGUI_VERTEX_LAYOUT,
                )?;

            let command_buffer = &write_context.command_buffer;
            command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

            self.per_pass_descriptor_set.bind(command_buffer)?; // view/projection
            self.per_image_descriptor_sets[0].bind(command_buffer)?; // font atlas
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
        ImGuiRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        ImGuiRenderFeature::feature_index()
    }
}
