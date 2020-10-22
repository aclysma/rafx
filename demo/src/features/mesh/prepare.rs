use renderer::nodes::{
    RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex,
    FramePacket, DefaultPrepareJobImpl, PerFrameNode, PerViewNode, RenderFeature,
};
use crate::features::mesh::{
    MeshRenderFeature, ExtractedFrameNodeMeshData, ExtractedViewNodeMeshData,
    PreparedViewNodeMeshData,
};
use crate::phases::OpaqueRenderPhase;
use glam::Vec3;
use super::MeshCommandWriter;
use crate::render_contexts::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer::assets::resources::{DescriptorSetArc, ResourceArc, GraphicsPipelineResource};

pub struct MeshPrepareJobImpl {
    pipeline_info: ResourceArc<GraphicsPipelineResource>,
    descriptor_sets_per_view: Vec<DescriptorSetArc>,
    extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    extracted_view_node_mesh_data: Vec<Vec<Option<ExtractedViewNodeMeshData>>>,
    prepared_view_node_mesh_data: Vec<PreparedViewNodeMeshData>,
}

impl MeshPrepareJobImpl {
    pub(super) fn new(
        pipeline_info: ResourceArc<GraphicsPipelineResource>,
        descriptor_sets_per_view: Vec<DescriptorSetArc>,
        extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
        extracted_view_node_mesh_data: Vec<Vec<Option<ExtractedViewNodeMeshData>>>,
    ) -> Self {
        let prepared_view_node_mesh_data = Vec::with_capacity(extracted_view_node_mesh_data.len());
        MeshPrepareJobImpl {
            pipeline_info,
            descriptor_sets_per_view,
            extracted_frame_node_mesh_data,
            extracted_view_node_mesh_data,
            prepared_view_node_mesh_data,
        }
    }
}

impl DefaultPrepareJobImpl<RenderJobPrepareContext, RenderJobWriteContext> for MeshPrepareJobImpl {
    fn prepare_begin(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {
    }

    fn prepare_frame_node(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        _frame_node: PerFrameNode,
        _frame_node_index: u32,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {
    }

    fn prepare_view_node(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    ) {
        let frame_node_index = view_node.frame_node_index();
        let extracted_frame_data = &self.extracted_frame_node_mesh_data[frame_node_index as usize];

        if let Some(extracted_frame_data) = extracted_frame_data {
            if let Some(extracted_view_data) = &self.extracted_view_node_mesh_data
                [view.view_index() as usize][view_node_index as usize]
            {
                let submit_node_id = self.prepared_view_node_mesh_data.len() as u32;
                self.prepared_view_node_mesh_data
                    .push(PreparedViewNodeMeshData {
                        per_view_descriptor: self.descriptor_sets_per_view
                            [view.view_index() as usize]
                            .clone(),
                        frame_node_index,
                        per_instance_descriptor: extracted_view_data
                            .per_instance_descriptor
                            .clone(),
                    });

                let distance_from_camera = Vec3::length(
                    extracted_frame_data.world_transform.w_axis().truncate() - view.eye_position(),
                );
                submit_nodes.add_submit_node::<OpaqueRenderPhase>(
                    submit_node_id,
                    0,
                    distance_from_camera,
                );
            }
        }
    }

    fn prepare_view_finalize(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        _view: &RenderView,
        _submit_nodes: &mut ViewSubmitNodes,
    ) {
    }

    fn prepare_frame_finalize(
        self,
        _prepare_context: &RenderJobPrepareContext,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<RenderJobWriteContext>> {
        Box::new(MeshCommandWriter {
            pipeline_info: self.pipeline_info,
            descriptor_sets_per_view: self.descriptor_sets_per_view,
            extracted_frame_node_mesh_data: self.extracted_frame_node_mesh_data,
            prepared_view_node_mesh_data: self.prepared_view_node_mesh_data,
        })
    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}
