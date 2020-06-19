use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use renderer_base::{RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex, FramePacket, DefaultPrepareJobImpl, PerFrameNode, PerViewNode, RenderFeature};
use crate::features::mesh::{MeshRenderFeature, ExtractedFrameNodeMeshData, MeshDrawCall, ExtractedViewNodeMeshData};
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use glam::Vec3;
use super::MeshCommandWriter;
use crate::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer_shell_vulkan::{VkBuffer, VkDeviceContext};
use ash::vk;
use std::mem::ManuallyDrop;
use crate::resource_managers::{PipelineSwapchainInfo, DescriptorSetArc};
use crate::pipeline::gltf::MeshVertex;

pub struct MeshPrepareJobImpl {
    device_context: VkDeviceContext,
    pipeline_info: PipelineSwapchainInfo,
    descriptor_set_per_pass: DescriptorSetArc,
    extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    extracted_view_node_mesh_data: Vec<Option<ExtractedViewNodeMeshData>>,
}

impl MeshPrepareJobImpl {
    pub(super) fn new(
        device_context: VkDeviceContext,
        pipeline_info: PipelineSwapchainInfo,
        descriptor_set_per_pass: DescriptorSetArc,
        extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
        extracted_view_node_mesh_data: Vec<Option<ExtractedViewNodeMeshData>>,
    ) -> Self {
        MeshPrepareJobImpl {
            device_context,
            pipeline_info,
            descriptor_set_per_pass,
            extracted_frame_node_mesh_data,
            extracted_view_node_mesh_data
        }
    }
}

impl DefaultPrepareJobImpl<RenderJobPrepareContext, RenderJobWriteContext> for MeshPrepareJobImpl {
    fn prepare_begin(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        _views: &[&RenderView],
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {

    }

    fn prepare_frame_node(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        _frame_node: PerFrameNode,
        frame_node_index: u32,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {

    }

    fn prepare_view_node(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    ) {
        let frame_node_index = view_node.frame_node_index();
        let extracted_data =
            &self.extracted_frame_node_mesh_data[frame_node_index as usize];

        //TODO: calculate distance
        if let Some(extracted_data) = extracted_data {
            let distance_from_camera = Vec3::length(extracted_data.world_transform.w_axis().truncate() - view.eye_position());
            submit_nodes.add_submit_node::<DrawOpaqueRenderPhase>(
                view_node_index,
                0,
                distance_from_camera,
            );
        }
    }

    fn prepare_view_finalize(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        _view: &RenderView,
        _submit_nodes: &mut ViewSubmitNodes,
    ) {

    }

    fn prepare_frame_finalize(
        self,
        prepare_context: &RenderJobPrepareContext,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<RenderJobWriteContext>> {
        Box::new(MeshCommandWriter {
            pipeline_info: self.pipeline_info,
            descriptor_set_per_pass: self.descriptor_set_per_pass,
            extracted_frame_node_mesh_data: self.extracted_frame_node_mesh_data,
            extracted_view_node_mesh_data: self.extracted_view_node_mesh_data
        })
    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}
