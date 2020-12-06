use crate::features::imgui::ImGuiRenderFeature;
use crate::imgui_support::{ImGuiDrawCmd, ImGuiDrawData};
use crate::render_contexts::RenderJobWriteContext;
use ash::version::DeviceV1_0;
use ash::vk;
use rafx::nodes::{
    FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderPhaseIndex, RenderView,
    SubmitNodeId,
};
use rafx::resources::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct ImGuiCommandWriter {
    pub(super) vertex_buffers: Vec<ResourceArc<BufferResource>>,
    pub(super) index_buffers: Vec<ResourceArc<BufferResource>>,
    pub(super) imgui_draw_data: Option<ImGuiDrawData>,
    pub(super) per_pass_descriptor_set: DescriptorSetArc,
    pub(super) per_image_descriptor_sets: Vec<DescriptorSetArc>,
    pub(super) imgui_material_pass: ResourceArc<MaterialPassResource>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for ImGuiCommandWriter {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
        if self.imgui_draw_data.is_some() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    &self.imgui_material_pass,
                    &write_context.renderpass,
                    &write_context.framebuffer_meta,
                    &*super::IMGUI_VERTEX_LAYOUT,
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
                    0,
                    &[
                        self.per_pass_descriptor_set.get(),      // view/projection
                        self.per_image_descriptor_sets[0].get(), // font atlas
                    ],
                    &[],
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
                let mut draw_list_index = 0;
                if let Some(draw_data) = &self.imgui_draw_data {
                    for draw_list in draw_data.draw_lists() {
                        logical_device.cmd_bind_vertex_buffers(
                            command_buffer,
                            0, // first binding
                            &[self.vertex_buffers[draw_list_index].get_raw().buffer.buffer],
                            &[0], // offsets
                        );

                        logical_device.cmd_bind_index_buffer(
                            command_buffer,
                            self.index_buffers[draw_list_index].get_raw().buffer.buffer,
                            0, // offset
                            vk::IndexType::UINT16,
                        );

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

                                    let scissors = vk::Rect2D {
                                        offset: vk::Offset2D {
                                            x: ((clip_rect[0] - draw_data.display_pos[0])
                                                * draw_data.framebuffer_scale[0])
                                                as i32,
                                            y: ((clip_rect[1] - draw_data.display_pos[1])
                                                * draw_data.framebuffer_scale[1])
                                                as i32,
                                        },
                                        extent: vk::Extent2D {
                                            width: ((clip_rect[2]
                                                - clip_rect[0]
                                                - draw_data.display_pos[0])
                                                * draw_data.framebuffer_scale[0])
                                                as u32,
                                            height: ((clip_rect[3]
                                                - clip_rect[1]
                                                - draw_data.display_pos[1])
                                                * draw_data.framebuffer_scale[1])
                                                as u32,
                                        },
                                    };

                                    logical_device.cmd_set_scissor(command_buffer, 0, &[scissors]);

                                    logical_device.cmd_draw_indexed(
                                        command_buffer,
                                        element_end_index - element_begin_index,
                                        1,
                                        element_begin_index,
                                        0,
                                        0,
                                    );

                                    element_begin_index = element_end_index;
                                }
                                _ => panic!("unexpected draw command"),
                            }
                        }

                        draw_list_index += 1;
                    }
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
        ImGuiRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        ImGuiRenderFeature::feature_index()
    }
}
