use rafx::render_feature_write_job_prelude::*;

use super::*;
use crate::phases::OpaqueRenderPhase;
use rafx::api::RafxPrimitiveTopology;
use rafx::api::{RafxIndexBufferBinding, RafxIndexType, RafxVertexBufferBinding};
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Vertex format for vertices sent to the GPU
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    // w component is a sign value (-1 or +1) indicating handedness of the tangent basis
    // see GLTF spec for more info
    pub tangent: [f32; 4],
    pub tex_coord: [f32; 2],
}

lazy_static::lazy_static! {
    pub static ref MESH_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&MeshVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.normal, "NORMAL", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.tangent, "TANGENT", RafxFormat::R32G32B32A32_SFLOAT);
            builder.add_member(&vertex.tex_coord, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

pub struct MeshWriteJob<'write> {
    depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    mesh_part_descriptor_sets: Arc<AtomicOnceCellStack<MeshPartDescriptorSetPair>>,
    frame_packet: Box<MeshFramePacket>,
    submit_packet: Box<MeshSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> MeshWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<MeshFramePacket>,
        submit_packet: Box<MeshSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            depth_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .depth_material_pass
                    .clone()
            },
            mesh_part_descriptor_sets: {
                submit_packet
                    .per_frame_submit_data()
                    .get()
                    .mesh_part_descriptor_sets
                    .clone()
            },
            frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for MeshWriteJob<'write> {
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

        let is_opaque_render_phase = render_phase_index == OpaqueRenderPhase::render_phase_index();
        let view_submit_packet = self.submit_packet.view_submit_packet(view_frame_index);
        let command_buffer = &write_context.command_buffer;

        let submit_node_data = view_submit_packet
            .get_submit_node_data_from_render_phase(render_phase_index, submit_node_id);
        let mesh_asset = &submit_node_data.mesh_asset;
        let mesh_part_index = submit_node_data.mesh_part_index;
        let mesh_part_descriptor_set_index = submit_node_data.mesh_part_descriptor_set_index;
        let mesh_part = mesh_asset.inner.mesh_parts[mesh_part_index]
            .as_ref()
            .unwrap();

        // Bind the correct pipeline.

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                render_phase_index,
                if is_opaque_render_phase {
                    &mesh_part.opaque_pass.material_pass_resource
                } else {
                    self.depth_material_pass.as_ref().unwrap()
                },
                &write_context.render_target_meta,
                &*MESH_VERTEX_LAYOUT,
            )?;

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        let per_view_submit_data = view_submit_packet.per_view_submit_data().get();
        let per_instance_descriptor_sets = self
            .mesh_part_descriptor_sets
            .get(mesh_part_descriptor_set_index + mesh_part_index);

        if is_opaque_render_phase {
            per_view_submit_data
                .opaque_descriptor_set
                .as_ref()
                .unwrap()
                .bind(command_buffer)?;
            mesh_part
                .opaque_material_descriptor_set
                .bind(command_buffer)?;
            per_instance_descriptor_sets
                .opaque_descriptor_set
                .bind(command_buffer)?;
        } else {
            per_view_submit_data
                .depth_descriptor_set
                .as_ref()
                .unwrap()
                .bind(command_buffer)?;
            per_instance_descriptor_sets
                .depth_descriptor_set
                .bind(command_buffer)?;
        }

        command_buffer.cmd_bind_vertex_buffers(
            0,
            &[RafxVertexBufferBinding {
                buffer: &mesh_asset.inner.vertex_buffer.get_raw().buffer,
                byte_offset: mesh_part.vertex_buffer_offset_in_bytes as u64,
            }],
        )?;

        command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
            buffer: &mesh_asset.inner.index_buffer.get_raw().buffer,
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

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
