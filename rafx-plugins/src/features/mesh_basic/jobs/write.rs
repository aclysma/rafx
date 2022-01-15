use rafx::render_feature_write_job_prelude::*;

use super::*;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase, WireframeRenderPhase,
};
use rafx::api::{RafxIndexBufferBinding, RafxVertexAttributeRate, RafxVertexBufferBinding};
use rafx::api::{RafxIndexType, RafxPrimitiveTopology};
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Vertex format for vertices sent to the GPU
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct MeshVertexFull {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub binormal: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct MeshVertexPosition {
    pub position: [f32; 3],
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct MeshModelMatrix {
    pub model_matrix: [[f32; 4]; 4],
}

lazy_static::lazy_static! {
    pub static ref MESH_VERTEX_FULL_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let per_vertex = VertexDataLayout::build_vertex_layout(&MeshVertexFull::default(), RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.normal, "NORMAL", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.tangent, "TANGENT", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.binormal, "BINORMAL", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.tex_coord, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
        });

        let per_instance = VertexDataLayout::build_vertex_layout(&MeshModelMatrix::default(), RafxVertexAttributeRate::Instance,  |builder, vertex| {
            builder.add_member(&vertex.model_matrix[0], "MODELMATRIX0", RafxFormat::R32G32B32A32_SFLOAT);
            builder.add_member(&vertex.model_matrix[1], "MODELMATRIX1", RafxFormat::R32G32B32A32_SFLOAT);
            builder.add_member(&vertex.model_matrix[2], "MODELMATRIX2", RafxFormat::R32G32B32A32_SFLOAT);
            builder.add_member(&vertex.model_matrix[3], "MODELMATRIX3", RafxFormat::R32G32B32A32_SFLOAT);
        });

        VertexDataSetLayout::new(vec![per_vertex, per_instance], RafxPrimitiveTopology::TriangleList)
    };

    pub static ref MESH_VERTEX_POSITION_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let per_vertex = VertexDataLayout::build_vertex_layout(&MeshVertexPosition::default(), RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
        });

        let per_instance = VertexDataLayout::build_vertex_layout(&MeshModelMatrix::default(), RafxVertexAttributeRate::Instance,  |builder, vertex| {
            builder.add_member(&vertex.model_matrix[0], "MODELMATRIX0", RafxFormat::R32G32B32A32_SFLOAT);
            builder.add_member(&vertex.model_matrix[1], "MODELMATRIX1", RafxFormat::R32G32B32A32_SFLOAT);
            builder.add_member(&vertex.model_matrix[2], "MODELMATRIX2", RafxFormat::R32G32B32A32_SFLOAT);
            builder.add_member(&vertex.model_matrix[3], "MODELMATRIX3", RafxFormat::R32G32B32A32_SFLOAT);
        });

        VertexDataSetLayout::new(vec![per_vertex, per_instance], RafxPrimitiveTopology::TriangleList)
    };
}

pub struct MeshBasicWriteJob<'write> {
    depth_prepass_index: RenderPhaseIndex,
    shadow_map_index: RenderPhaseIndex,
    wireframe_index: RenderPhaseIndex,
    frame_packet: Box<MeshBasicFramePacket>,
    submit_packet: Box<MeshSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> MeshBasicWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<MeshBasicFramePacket>,
        submit_packet: Box<MeshSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        let depth_prepass_index = DepthPrepassRenderPhase::render_phase_index();
        let shadow_map_index = ShadowMapRenderPhase::render_phase_index();
        let wireframe_index = WireframeRenderPhase::render_phase_index();

        Arc::new(Self {
            depth_prepass_index,
            shadow_map_index,
            wireframe_index,
            frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for MeshBasicWriteJob<'write> {
    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> u32 {
        self.frame_packet.view_frame_index(view)
    }

    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        view_frame_index: ViewFrameIndex,
        render_phase_index: RenderPhaseIndex,
        submit_node_id: SubmitNodeId,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        // Bailing here about 118 fps

        let is_wireframe = render_phase_index == self.wireframe_index;
        let is_depth_render_phase = render_phase_index == self.depth_prepass_index
            || render_phase_index == self.shadow_map_index;

        let view_submit_packet = self.submit_packet.view_submit_packet(view_frame_index);

        let command_buffer = &write_context.command_buffer;

        let model_matrix_buffer = self
            .submit_packet
            .per_frame_submit_data()
            .get()
            .model_matrix_buffer
            .borrow();

        let submit_node_data = view_submit_packet
            .get_submit_node_data_from_render_phase(render_phase_index, submit_node_id);

        let render_object_instance = self
            .frame_packet
            .render_object_instances_data()
            .get(submit_node_data.render_object_instance_id as usize)
            .as_ref()
            .unwrap();

        let material_pass = &submit_node_data.material_pass_resource;
        let per_material_descriptor_set = &submit_node_data.per_material_descriptor_set;
        let mesh_part_index = submit_node_data.mesh_part_index;

        let mesh_asset = &render_object_instance.mesh_asset;
        let mesh_part = mesh_asset.inner.mesh_parts[mesh_part_index]
            .as_ref()
            .unwrap();

        // Bind the correct pipeline.
        let (mesh_vertex_layout, vertex_buffer, vertex_buffer_offset_in_bytes) =
            if render_phase_index == OpaqueRenderPhase::render_phase_index() {
                (
                    &*MESH_VERTEX_FULL_LAYOUT,
                    &mesh_asset.inner.vertex_full_buffer,
                    mesh_part.vertex_full_buffer_offset_in_bytes,
                )
            } else {
                (
                    &*MESH_VERTEX_POSITION_LAYOUT,
                    &mesh_asset.inner.vertex_position_buffer,
                    mesh_part.vertex_position_buffer_offset_in_bytes,
                )
            };

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                Some(render_phase_index),
                material_pass,
                &write_context.render_target_meta,
                mesh_vertex_layout,
            )?;

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        let per_view_submit_data = view_submit_packet.per_view_submit_data().get();

        if is_depth_render_phase || is_wireframe {
            per_view_submit_data
                .depth_descriptor_set
                .as_ref()
                .unwrap()
                .bind(command_buffer)?;
        } else {
            per_view_submit_data
                .opaque_descriptor_set
                .as_ref()
                .unwrap()
                .bind(command_buffer)?;
        }

        if let Some(per_material_descriptor_set) = per_material_descriptor_set {
            per_material_descriptor_set.bind(command_buffer)?;
        }

        command_buffer.cmd_bind_vertex_buffers(
            0,
            &[
                // NOTE(dvd): Bind the mesh vertex data.
                RafxVertexBufferBinding {
                    buffer: &vertex_buffer.get_raw().buffer,
                    byte_offset: vertex_buffer_offset_in_bytes as u64,
                },
                // NOTE(dvd): Bind the mesh model matrices. We pass these through instanced vertex
                // attributes instead of descriptor sets so that we don't spend CPU time managing
                // the descriptor sets. Another option would be push constants, but they aren't as
                // well supported on rafx backends. A third option would be some type of dynamic
                // uniform buffer, but we'd still need to pass in an index to the shader for each instance.
                RafxVertexBufferBinding {
                    buffer: &model_matrix_buffer.as_ref().unwrap().get_raw().buffer,
                    byte_offset: (std::mem::size_of::<MeshModelMatrix>()
                        * submit_node_data.model_matrix_offset)
                        as u64,
                },
            ],
        )?;

        command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
            buffer: &mesh_asset.inner.index_buffer.get_raw().buffer,
            byte_offset: mesh_part.index_buffer_offset_in_bytes as u64,
            index_type: mesh_part.index_type,
        })?;

        let index_size = match mesh_part.index_type {
            RafxIndexType::Uint16 => std::mem::size_of::<u16>(),
            RafxIndexType::Uint32 => std::mem::size_of::<u32>(),
        } as u32;

        command_buffer.cmd_draw_indexed(mesh_part.index_buffer_size_in_bytes / index_size, 0, 0)?;

        Ok(())
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
