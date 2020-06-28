use crate::phases::UiRenderPhase;
use renderer::nodes::{
    RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex,
    FramePacket, RenderFeature, PrepareJob,
};
use crate::features::imgui::{ImGuiRenderFeature, ExtractedImGuiData};
use super::write::ImGuiCommandWriter;
use crate::render_contexts::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer::vulkan::{VkBuffer, VkDeviceContext};
use ash::vk;
use renderer::assets::resource_managers::{PipelineSwapchainInfo, DescriptorSetArc};

pub struct ImGuiPrepareJobImpl {
    device_context: VkDeviceContext,
    pipeline_info: PipelineSwapchainInfo,
    dyn_resource_allocator: renderer::assets::DynResourceAllocatorSet,
    per_pass_descriptor_set: DescriptorSetArc,
    per_image_descriptor_sets: Vec<DescriptorSetArc>,
    extracted_imgui_data: ExtractedImGuiData,
}

impl ImGuiPrepareJobImpl {
    pub(super) fn new(
        device_context: VkDeviceContext,
        pipeline_info: PipelineSwapchainInfo,
        dyn_resource_allocator: renderer::assets::DynResourceAllocatorSet,
        per_pass_descriptor_set: DescriptorSetArc,
        per_image_descriptor_sets: Vec<DescriptorSetArc>,
        extracted_imgui_data: ExtractedImGuiData,
    ) -> Self {
        ImGuiPrepareJobImpl {
            device_context,
            pipeline_info,
            dyn_resource_allocator,
            per_pass_descriptor_set,
            per_image_descriptor_sets,
            extracted_imgui_data,
        }
    }
}

impl PrepareJob<RenderJobPrepareContext, RenderJobWriteContext> for ImGuiPrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        _prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (
        Box<dyn FeatureCommandWriter<RenderJobWriteContext>>,
        FeatureSubmitNodes,
    ) {
        let draw_list_count = self
            .extracted_imgui_data
            .imgui_draw_data
            .as_ref()
            .unwrap()
            .draw_lists()
            .len();
        let mut vertex_buffers = Vec::with_capacity(draw_list_count);
        let mut index_buffers = Vec::with_capacity(draw_list_count);
        if let Some(draw_data) = &self.extracted_imgui_data.imgui_draw_data {
            for draw_list in draw_data.draw_lists() {
                let vertex_buffer_size = draw_list.vertex_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawVert>() as u64;
                let mut vertex_buffer = VkBuffer::new(
                    &self.device_context,
                    vk_mem::MemoryUsage::CpuOnly,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    vertex_buffer_size,
                )
                .unwrap();
                vertex_buffer
                    .write_to_host_visible_buffer(draw_list.vertex_buffer())
                    .unwrap();
                let vertex_buffer = self.dyn_resource_allocator.insert_buffer(vertex_buffer);

                let index_buffer_size = draw_list.index_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawIdx>() as u64;
                let mut index_buffer = VkBuffer::new(
                    &self.device_context,
                    vk_mem::MemoryUsage::CpuOnly,
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    index_buffer_size,
                )
                .unwrap();

                index_buffer
                    .write_to_host_visible_buffer(draw_list.index_buffer())
                    .unwrap();
                let index_buffer = self.dyn_resource_allocator.insert_buffer(index_buffer);

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
            pipeline_info: self.pipeline_info,
            per_pass_descriptor_set: self.per_pass_descriptor_set,
            per_image_descriptor_sets: self.per_image_descriptor_sets,
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
