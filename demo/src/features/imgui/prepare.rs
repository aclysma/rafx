use super::write::ImGuiCommandWriter;
use crate::features::imgui::{ExtractedImGuiData, ImGuiRenderFeature, ImGuiUniformBufferObject};
use crate::phases::UiRenderPhase;
use crate::render_contexts::{RenderJobPrepareContext, RenderJobWriteContext};
use ash::vk;
use renderer::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PrepareJob, RenderFeature,
    RenderFeatureIndex, RenderView, ViewSubmitNodes,
};
use renderer::resources::{ImageViewResource, MaterialPassResource, ResourceArc};
use renderer::vulkan::VkBuffer;

pub struct ImGuiPrepareJobImpl {
    extracted_imgui_data: ExtractedImGuiData,
    imgui_material_pass: ResourceArc<MaterialPassResource>,
    view_ubo: ImGuiUniformBufferObject,
    font_atlas: ResourceArc<ImageViewResource>,
}

impl ImGuiPrepareJobImpl {
    pub(super) fn new(
        extracted_imgui_data: ExtractedImGuiData,
        imgui_material_pass: ResourceArc<MaterialPassResource>,
        view_ubo: ImGuiUniformBufferObject,
        font_atlas: ResourceArc<ImageViewResource>,
    ) -> Self {
        ImGuiPrepareJobImpl {
            extracted_imgui_data,
            imgui_material_pass,
            view_ubo,
            font_atlas,
        }
    }
}

impl PrepareJob<RenderJobPrepareContext, RenderJobWriteContext> for ImGuiPrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (
        Box<dyn FeatureCommandWriter<RenderJobWriteContext>>,
        FeatureSubmitNodes,
    ) {
        profiling::scope!("ImGui Prepare");

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();
        let dyn_resource_allocator = prepare_context
            .resource_context
            .create_dyn_resource_allocator_set();
        let draw_list_count = self
            .extracted_imgui_data
            .imgui_draw_data
            .as_ref()
            .unwrap()
            .draw_lists()
            .len();

        let descriptor_set_layouts = &self
            .imgui_material_pass
            .get_raw()
            .pipeline_layout
            .get_raw()
            .descriptor_sets;

        let per_pass_descriptor_set = descriptor_set_allocator
            .create_descriptor_set(
                &descriptor_set_layouts[shaders::imgui_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX],
                shaders::imgui_vert::DescriptorSet0Args {
                    uniform_buffer: &self.view_ubo,
                },
            )
            .unwrap();

        let per_image_descriptor_set = descriptor_set_allocator
            .create_descriptor_set(
                &descriptor_set_layouts[shaders::imgui_frag::TEX_DESCRIPTOR_SET_INDEX],
                shaders::imgui_frag::DescriptorSet1Args {
                    tex: &self.font_atlas,
                },
            )
            .unwrap();

        let per_image_descriptor_sets = vec![per_image_descriptor_set];

        let mut vertex_buffers = Vec::with_capacity(draw_list_count);
        let mut index_buffers = Vec::with_capacity(draw_list_count);
        if let Some(draw_data) = &self.extracted_imgui_data.imgui_draw_data {
            for draw_list in draw_data.draw_lists() {
                let vertex_buffer_size = draw_list.vertex_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawVert>() as u64;
                let mut vertex_buffer = VkBuffer::new(
                    &prepare_context.device_context,
                    vk_mem::MemoryUsage::CpuOnly,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    vertex_buffer_size,
                )
                .unwrap();
                vertex_buffer
                    .write_to_host_visible_buffer(draw_list.vertex_buffer())
                    .unwrap();
                let vertex_buffer = dyn_resource_allocator.insert_buffer(vertex_buffer);

                let index_buffer_size = draw_list.index_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawIdx>() as u64;
                let mut index_buffer = VkBuffer::new(
                    &prepare_context.device_context,
                    vk_mem::MemoryUsage::CpuOnly,
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    index_buffer_size,
                )
                .unwrap();

                index_buffer
                    .write_to_host_visible_buffer(draw_list.index_buffer())
                    .unwrap();
                let index_buffer = dyn_resource_allocator.insert_buffer(index_buffer);

                vertex_buffers.push(vertex_buffer);
                index_buffers.push(index_buffer);
            }
        }

        //
        // Submit a single node for each view
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());
            view_submit_nodes.add_submit_node::<UiRenderPhase>(0, 0, 0.0);
            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        let writer = Box::new(ImGuiCommandWriter {
            imgui_draw_data: self.extracted_imgui_data.imgui_draw_data,
            vertex_buffers,
            index_buffers,
            per_pass_descriptor_set,
            per_image_descriptor_sets,
            imgui_material_pass: self.imgui_material_pass,
        });

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        ImGuiRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        ImGuiRenderFeature::feature_index()
    }
}
