use crate::phases::UiRenderPhase;
use renderer::nodes::{
    RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex,
    FramePacket, RenderFeature, PrepareJob,
};
use crate::features::imgui::{ImGuiRenderFeature, ExtractedImGuiData, ImGuiUniformBufferObject};
use super::write::ImGuiCommandWriter;
use crate::render_contexts::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer::vulkan::VkBuffer;
use ash::vk;
use renderer::assets::resources::{ResourceArc, MaterialPassResource, ImageViewResource};

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

        let per_pass_layout = &self
            .imgui_material_pass
            .get_raw()
            .pipeline_layout
            .get_raw()
            .descriptor_sets[0];
        let mut per_pass_descriptor_set = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(&per_pass_layout)
            .unwrap();
        per_pass_descriptor_set.set_buffer_data(0, &self.view_ubo);
        per_pass_descriptor_set
            .flush(&mut descriptor_set_allocator)
            .unwrap();

        let per_image_layout = &self
            .imgui_material_pass
            .get_raw()
            .pipeline_layout
            .get_raw()
            .descriptor_sets[1];
        let mut per_image_descriptor_set = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(&per_image_layout)
            .unwrap();
        per_image_descriptor_set.set_image(0, self.font_atlas.clone());
        per_image_descriptor_set
            .flush(&mut descriptor_set_allocator)
            .unwrap();

        let per_pass_descriptor_set = per_pass_descriptor_set.descriptor_set().clone();
        let per_image_descriptor_sets = vec![per_image_descriptor_set.descriptor_set().clone()];

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
