use rafx::render_feature_write_job_prelude::*;

use crate::features::mesh::prepare::ExtractedFrameNodeMeshData;
use rafx::api::{RafxIndexBufferBinding, RafxIndexType, RafxVertexBufferBinding};
use rafx::framework::{DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::nodes::{FrameNodeIndex, PerViewNode};

struct PreparedSubmitNodeMeshData {
    material_pass_resource: ResourceArc<MaterialPassResource>,
    per_view_descriptor_set: DescriptorSetArc,
    per_material_descriptor_set: Option<DescriptorSetArc>,
    per_instance_descriptor_set: DescriptorSetArc,
    // we can get the mesh via the frame node index
    frame_node_index: FrameNodeIndex,
    mesh_part_index: usize,
}

impl std::fmt::Debug for PreparedSubmitNodeMeshData {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("PreparedSubmitNodeMeshData")
            .field("frame_node_index", &self.frame_node_index)
            .field("mesh_part_index", &self.mesh_part_index)
            .finish()
    }
}

pub struct WriteJobImpl {
    extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    prepared_submit_node_mesh_data: Vec<PreparedSubmitNodeMeshData>,
}

impl WriteJobImpl {
    pub fn new() -> Self {
        WriteJobImpl {
            extracted_frame_node_mesh_data: Default::default(),
            prepared_submit_node_mesh_data: Default::default(),
        }
    }

    pub fn push_submit_node(
        &mut self,
        view_node: &PerViewNode,
        per_view_descriptor_set: DescriptorSetArc,
        per_material_descriptor_set: Option<DescriptorSetArc>,
        per_instance_descriptor_set: DescriptorSetArc,
        mesh_part_index: usize,
        material_pass_resource: ResourceArc<MaterialPassResource>,
    ) -> usize {
        let submit_node_index = self.prepared_submit_node_mesh_data.len();
        self.prepared_submit_node_mesh_data
            .push(PreparedSubmitNodeMeshData {
                material_pass_resource: material_pass_resource.clone(),
                per_view_descriptor_set,
                per_material_descriptor_set,
                per_instance_descriptor_set,
                frame_node_index: view_node.frame_node_index(),
                mesh_part_index,
            });
        submit_node_index
    }

    pub fn set_extracted_frame_node_mesh_data(
        &mut self,
        extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    ) {
        self.extracted_frame_node_mesh_data = extracted_frame_node_mesh_data;
    }
}

impl FeatureCommandWriter for WriteJobImpl {
    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_element_scope);

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
                &render_node_data.material_pass_resource,
                &write_context.render_target_meta,
                &*crate::assets::gltf::MESH_VERTEX_LAYOUT,
            )?;

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        render_node_data
            .per_view_descriptor_set
            .bind(command_buffer)?;

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
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
