rafx::declare_render_feature_prepare_job!();

use super::internal::ImGuiDrawData;
use crate::phases::UiRenderPhase;
use rafx::api::{RafxBufferDef, RafxMemoryUsage, RafxResourceType};
use rafx::framework::{ImageViewResource, MaterialPassResource, ResourceArc};

/// Per-pass "global" data
pub type ImGuiUniformBufferObject = shaders::imgui_vert::ArgsUniform;

pub struct PrepareJobImpl {
    imgui_draw_data: Option<ImGuiDrawData>,
    imgui_material_pass: ResourceArc<MaterialPassResource>,
    view_ubo: ImGuiUniformBufferObject,
    font_atlas: ResourceArc<ImageViewResource>,
}

impl PrepareJobImpl {
    pub(super) fn new(
        imgui_draw_data: Option<ImGuiDrawData>,
        imgui_material_pass: ResourceArc<MaterialPassResource>,
        view_ubo: ImGuiUniformBufferObject,
        font_atlas: ResourceArc<ImageViewResource>,
    ) -> Self {
        PrepareJobImpl {
            imgui_draw_data,
            imgui_material_pass,
            view_ubo,
            font_atlas,
        }
    }
}

impl PrepareJob for PrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes) {
        profiling::scope!(prepare_scope);

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        let dyn_resource_allocator = prepare_context
            .resource_context
            .create_dyn_resource_allocator_set();

        let descriptor_set_layouts = &self.imgui_material_pass.get_raw().descriptor_set_layouts;

        let per_view_descriptor_set = descriptor_set_allocator
            .create_descriptor_set(
                &descriptor_set_layouts[shaders::imgui_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX],
                shaders::imgui_vert::DescriptorSet0Args {
                    uniform_buffer: &self.view_ubo,
                },
            )
            .unwrap();

        let per_font_descriptor_set = descriptor_set_allocator
            .create_descriptor_set(
                &descriptor_set_layouts[shaders::imgui_frag::TEX_DESCRIPTOR_SET_INDEX],
                shaders::imgui_frag::DescriptorSet1Args {
                    tex: &self.font_atlas,
                },
            )
            .unwrap();

        let mut writer = Box::new(FeatureCommandWriterImpl::new(
            self.imgui_material_pass.clone(),
            per_view_descriptor_set,
            per_font_descriptor_set,
            self.imgui_draw_data
                .as_ref()
                .map_or(0, |draw_data| draw_data.draw_lists().len()),
        ));

        if let Some(draw_data) = &self.imgui_draw_data {
            for draw_list in draw_data.draw_lists() {
                let vertex_buffer_size = draw_list.vertex_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawVert>() as u64;

                let vertex_buffer = prepare_context
                    .device_context
                    .create_buffer(&RafxBufferDef {
                        size: vertex_buffer_size,
                        memory_usage: RafxMemoryUsage::CpuToGpu,
                        resource_type: RafxResourceType::VERTEX_BUFFER,
                        ..Default::default()
                    })
                    .unwrap();

                vertex_buffer
                    .copy_to_host_visible_buffer(draw_list.vertex_buffer())
                    .unwrap();

                let vertex_buffer = dyn_resource_allocator.insert_buffer(vertex_buffer);

                let index_buffer_size = draw_list.index_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawIdx>() as u64;

                let index_buffer = prepare_context
                    .device_context
                    .create_buffer(&RafxBufferDef {
                        size: index_buffer_size,
                        memory_usage: RafxMemoryUsage::CpuToGpu,
                        resource_type: RafxResourceType::INDEX_BUFFER,
                        ..Default::default()
                    })
                    .unwrap();

                index_buffer
                    .copy_to_host_visible_buffer(draw_list.index_buffer())
                    .unwrap();

                let index_buffer = dyn_resource_allocator.insert_buffer(index_buffer);

                writer.push_buffers(vertex_buffer, index_buffer);
            }
        }

        writer.set_imgui_draw_data(self.imgui_draw_data);

        //
        // Submit a single node for each view
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(render_feature_index(), view.render_phase_mask());
            view_submit_nodes.add_submit_node::<UiRenderPhase>(0, 0, 0.0);
            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }
}
