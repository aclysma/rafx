use crate::features::mesh::{
    ExtractedFrameNodeMeshData, MeshRenderFeature, PreparedSubmitNodeMeshData,
};
use rafx::api::{RafxIndexBufferBinding, RafxIndexType, RafxResult, RafxVertexBufferBinding};
use rafx::nodes::{FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderPhaseIndex, RenderView, SubmitNodeId, RenderJobWriteContext};

pub struct MeshCommandWriter {
    pub(super) extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub(super) prepared_submit_node_mesh_data: Vec<PreparedSubmitNodeMeshData>,
}

impl FeatureCommandWriter for MeshCommandWriter {
    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> RafxResult<()> {
        let command_buffer = &write_context.command_buffer;

        let render_node_data = &self.prepared_submit_node_mesh_data[index as usize];
        let frame_node_data: &ExtractedFrameNodeMeshData = self.extracted_frame_node_mesh_data
            [render_node_data.frame_node_index as usize]
            .as_ref()
            .unwrap();

        // Always valid, we don't generate render nodes for mesh parts that are None
        let mesh_part = &frame_node_data.mesh_asset.inner.mesh_parts
            [render_node_data.mesh_part_index]
            .as_ref()
            .unwrap();

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                render_phase_index,
                &render_node_data.material_pass.material_pass_resource,
                &write_context.render_target_meta,
                &*crate::assets::gltf::MESH_VERTEX_LAYOUT,
            )?;

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        // frag shader per-view data, not present during shadow pass
        if let Some(per_view_descriptor_set) = &render_node_data.per_view_descriptor_set {
            per_view_descriptor_set.bind(command_buffer).unwrap();
        };

        // frag shader material data, not present during shadow pass
        if let Some(per_material_descriptor_set) = &render_node_data.per_material_descriptor_set {
            per_material_descriptor_set.bind(command_buffer).unwrap();
        }

        render_node_data
            .per_instance_descriptor_set
            .bind(command_buffer)?;

        command_buffer.cmd_bind_vertex_buffers(
            0,
            &[RafxVertexBufferBinding {
                buffer: &frame_node_data
                    .mesh_asset
                    .inner
                    .vertex_buffer
                    .get_raw()
                    .buffer,
                byte_offset: mesh_part.vertex_buffer_offset_in_bytes as u64,
            }],
        )?;

        command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
            buffer: &frame_node_data
                .mesh_asset
                .inner
                .index_buffer
                .get_raw()
                .buffer,
            byte_offset: mesh_part.index_buffer_offset_in_bytes as u64,
            index_type: RafxIndexType::Uint16,
        })?;

        command_buffer.cmd_draw_indexed(
            mesh_part.index_buffer_size_in_bytes / 2, //sizeof(u16)
            0,
            0,
        )?;
        Ok(())
    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}
