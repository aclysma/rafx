use renderer::nodes::{
    RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex,
    FramePacket, RenderFeature, PrepareJob,
};
use crate::features::debug3d::{
    Debug3dRenderFeature, ExtractedDebug3dData, Debug3dDrawCall, Debug3dVertex,
};
use crate::phases::OpaqueRenderPhase;
use super::write::Debug3dCommandWriter;
use crate::render_contexts::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer::vulkan::{VkBuffer, VkDeviceContext};
use ash::vk;
use renderer::assets::resources::{DescriptorSetArc, ResourceArc, GraphicsPipelineResource};

pub struct Debug3dPrepareJobImpl {
    device_context: VkDeviceContext,
    pipeline_info: ResourceArc<GraphicsPipelineResource>,
    dyn_resource_allocator: renderer::assets::DynResourceAllocatorSet,
    descriptor_set_per_view: Vec<DescriptorSetArc>,
    extracted_debug3d_data: ExtractedDebug3dData,
}

impl Debug3dPrepareJobImpl {
    pub(super) fn new(
        device_context: VkDeviceContext,
        pipeline_info: ResourceArc<GraphicsPipelineResource>,
        dyn_resource_allocator: renderer::assets::DynResourceAllocatorSet,
        descriptor_set_per_view: Vec<DescriptorSetArc>,
        extracted_debug3d_data: ExtractedDebug3dData,
    ) -> Self {
        Debug3dPrepareJobImpl {
            device_context,
            pipeline_info,
            dyn_resource_allocator,
            descriptor_set_per_view,
            extracted_debug3d_data,
        }
    }
}

impl PrepareJob<RenderJobPrepareContext, RenderJobWriteContext> for Debug3dPrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        _prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (
        Box<dyn FeatureCommandWriter<RenderJobWriteContext>>,
        FeatureSubmitNodes,
    ) {
        //
        // Gather the raw draw data
        //
        let line_lists = &self.extracted_debug3d_data.line_lists;
        let mut draw_calls = Vec::with_capacity(line_lists.len());

        let mut vertex_list: Vec<Debug3dVertex> = vec![];
        for line_list in line_lists {
            let vertex_buffer_first_element = vertex_list.len() as u32;

            for vertex_pos in &line_list.points {
                vertex_list.push(Debug3dVertex {
                    pos: (*vertex_pos).into(),
                    color: line_list.color.into(),
                });
            }

            let draw_call = Debug3dDrawCall {
                first_element: vertex_buffer_first_element,
                count: line_list.points.len() as u32,
            };

            draw_calls.push(draw_call);
        }

        // We would probably want to support multiple buffers at some point
        let vertex_buffer = if !draw_calls.is_empty() {
            let vertex_buffer_size =
                vertex_list.len() as u64 * std::mem::size_of::<Debug3dVertex>() as u64;
            let mut vertex_buffer = VkBuffer::new(
                &self.device_context,
                vk_mem::MemoryUsage::CpuToGpu,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                vertex_buffer_size,
            )
            .unwrap();

            vertex_buffer
                .write_to_host_visible_buffer(vertex_list.as_slice())
                .unwrap();

            Some(self.dyn_resource_allocator.insert_buffer(vertex_buffer))
        } else {
            None
        };

        //
        // Submit a single node for each view
        // TODO: Submit separate nodes for transparency
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());
            view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(0, 0, 0.0);
            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        let writer = Box::new(Debug3dCommandWriter {
            draw_calls,
            vertex_buffer,
            pipeline_info: self.pipeline_info,
            descriptor_set_per_view: self.descriptor_set_per_view,
        });

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        Debug3dRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        Debug3dRenderFeature::feature_index()
    }
}
