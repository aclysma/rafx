use rafx::render_feature_write_job_prelude::*;

use super::*;
use crate::assets::mesh_adv::MeshAdvBindlessBuffers;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase, TransparentRenderPhase,
    WireframeRenderPhase,
};
use rafx::api::{RafxIndexBufferBinding, RafxVertexAttributeRate, RafxVertexBufferBinding};
use rafx::api::{RafxIndexedIndirectCommandSignature, RafxPrimitiveTopology};
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};
use rafx::render_features::{BeginSubmitNodeBatchArgs, RenderSubmitNodeArgs};
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
pub struct ShadowMapAtlasClearTileVertex {
    pub position: [f32; 2],
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct MeshModelMatrix {
    pub model_matrix: [[f32; 4]; 4],
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
#[repr(C)]
pub struct MeshModelMatrixWithHistory {
    pub current_model_matrix: [[f32; 4]; 4],
    pub previous_model_matrix: [[f32; 4]; 4],
}

lazy_static::lazy_static! {
    pub static ref MESH_FULL_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let per_vertex = VertexDataLayout::build_vertex_layout(&MeshVertexFull::default(), RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.normal, "NORMAL", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.tangent, "TANGENT", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.binormal, "BINORMAL", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.tex_coord, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
        });

        VertexDataSetLayout::new(vec![per_vertex], RafxPrimitiveTopology::TriangleList)
    };

    pub static ref MESH_POSITION_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let per_vertex = VertexDataLayout::build_vertex_layout(&MeshVertexPosition::default(), RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
        });

        VertexDataSetLayout::new(vec![per_vertex], RafxPrimitiveTopology::TriangleList)
    };

    pub static ref SHADOW_MAP_ATLAS_CLEAR_TILE_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        let per_vertex = VertexDataLayout::build_vertex_layout(&ShadowMapAtlasClearTileVertex::default(), RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32_SFLOAT);
        });

        VertexDataSetLayout::new(vec![per_vertex], RafxPrimitiveTopology::TriangleList)
    };
}

pub struct MeshAdvWriteJob<'write> {
    _frame_packet: Box<MeshAdvFramePacket>,
    submit_packet: Box<MeshSubmitPacket>,
    buffer_heaps: MeshAdvBindlessBuffers,
    pbr_indirect_signature: RafxIndexedIndirectCommandSignature,
    wireframe_indirect_signature: RafxIndexedIndirectCommandSignature,
    shadow_indirect_signature: RafxIndexedIndirectCommandSignature,
    depth_indirect_signature: RafxIndexedIndirectCommandSignature,
    phantom: PhantomData<&'write ()>,
}

impl<'write> MeshAdvWriteJob<'write> {
    pub fn new(
        write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<MeshAdvFramePacket>,
        submit_packet: Box<MeshSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        let buffer_heaps = (*write_context
            .render_resources
            .fetch::<MeshAdvBindlessBuffers>())
        .clone();

        let static_resources = write_context
            .render_resources
            .fetch::<MeshAdvStaticResources>();

        Arc::new(Self {
            _frame_packet: frame_packet,
            submit_packet,
            buffer_heaps,
            pbr_indirect_signature: static_resources.pbr_indirect_signature.clone(),
            wireframe_indirect_signature: static_resources.wireframe_indirect_signature.clone(),
            shadow_indirect_signature: static_resources.shadow_indirect_signature.clone(),
            depth_indirect_signature: static_resources.depth_indirect_signature.clone(),
            phantom: Default::default(),
        })
    }
}

impl<'write> MeshAdvWriteJob<'write> {
    fn vertex_layout_for_phase_index(phase_index: RenderPhaseIndex) -> &'write VertexDataSetLayout {
        if phase_index == OpaqueRenderPhase::render_phase_index()
            || phase_index == TransparentRenderPhase::render_phase_index()
        {
            &*MESH_FULL_LAYOUT
        } else {
            &*MESH_POSITION_LAYOUT
        }
    }

    fn setup_for_batch(
        &self,
        batch_index: u32,
        write_context: &mut RenderJobCommandBufferContext,
        render_phase_index: RenderPhaseIndex,
        view_submit_packet: &ViewSubmitPacket<MeshAdvRenderFeatureTypes>,
    ) -> RafxResult<()> {
        let per_view_submit_data = view_submit_packet.per_view_submit_data().get();

        let (per_view_descriptor_set, bind_ssao_and_materials) =
            if render_phase_index == ShadowMapRenderPhase::render_phase_index() {
                let per_view_descriptor_set = per_view_submit_data
                    .shadow_map_atlas_depth_descriptor_set
                    .as_ref()
                    .unwrap();

                (per_view_descriptor_set, false)
            } else if render_phase_index == DepthPrepassRenderPhase::render_phase_index() {
                let per_view_descriptor_set =
                    per_view_submit_data.depth_descriptor_set.as_ref().unwrap();

                (per_view_descriptor_set, false)
            } else if render_phase_index == WireframeRenderPhase::render_phase_index() {
                let per_view_descriptor_set =
                    per_view_submit_data.opaque_descriptor_set.as_ref().unwrap();

                (per_view_descriptor_set, false)
            } else if render_phase_index == OpaqueRenderPhase::render_phase_index()
                || render_phase_index == TransparentRenderPhase::render_phase_index()
            {
                let per_view_descriptor_set =
                    per_view_submit_data.opaque_descriptor_set.as_ref().unwrap();

                (per_view_descriptor_set, true)
            } else {
                panic!("Tried to render meshes as batch in unsupported render phase");
            };

        let vertex_layout = Self::vertex_layout_for_phase_index(render_phase_index);

        let command_buffer = &write_context.command_buffer;

        let batch = &self
            .submit_packet
            .per_frame_submit_data()
            .get()
            .batched_passes
            .get()[batch_index as usize];

        let per_batch_descriptor_set = self
            .submit_packet
            .per_frame_submit_data()
            .get()
            .per_batch_descriptor_sets
            .get()[batch_index as usize]
            .as_ref()
            .unwrap()
            .clone();

        let pipeline = write_context
            .resource_context()
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                Some(batch.phase),
                &batch.pass,
                &write_context.render_target_meta,
                &*vertex_layout,
            )?;

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        per_view_descriptor_set.bind(command_buffer)?;

        per_batch_descriptor_set.bind(command_buffer)?;

        //
        // Extra descriptor sets for meshes
        //
        if bind_ssao_and_materials {
            let ssao_descriptor_set = write_context
                .graph_context
                .render_resources()
                .fetch::<MeshAdvRenderPipelineState>()
                .ssao_descriptor_set
                .clone();

            if let Some(ssao_descriptor_set) = ssao_descriptor_set {
                ssao_descriptor_set.bind(command_buffer)?;
            }

            let all_materials_descriptor_set = (*self
                .submit_packet
                .per_frame_submit_data()
                .get()
                .all_materials_descriptor_set
                .borrow())
            .clone()
            .unwrap();
            all_materials_descriptor_set.bind(command_buffer)?;
        }

        command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
            buffer: &self.buffer_heaps.index.get_raw().buffer,
            byte_offset: 0,
            index_type: batch.index_type,
        })?;

        command_buffer.cmd_bind_vertex_buffers(
            0,
            &[RafxVertexBufferBinding {
                buffer: &self.buffer_heaps.vertex.get_raw().buffer,
                byte_offset: 0,
            }],
        )?;

        return Ok(());
    }

    fn indirect_signature(
        &self,
        render_phase_index: RenderPhaseIndex,
    ) -> &RafxIndexedIndirectCommandSignature {
        if render_phase_index == WireframeRenderPhase::render_phase_index() {
            &self.wireframe_indirect_signature
        } else if render_phase_index == ShadowMapRenderPhase::render_phase_index() {
            &self.shadow_indirect_signature
        } else if render_phase_index == DepthPrepassRenderPhase::render_phase_index() {
            &self.depth_indirect_signature
        } else {
            &self.pbr_indirect_signature
        }
    }

    fn draw_batch(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        render_phase_index: RenderPhaseIndex,
        submit_node_id: SubmitNodeId,
        view_submit_packet: &ViewSubmitPacket<MeshAdvRenderFeatureTypes>,
    ) -> RafxResult<()> {
        let batched_draw_call = view_submit_packet
            .get_submit_node_data_from_render_phase(render_phase_index, submit_node_id)
            .as_batched()
            .unwrap();

        self.setup_for_batch(
            batched_draw_call.batch_index,
            write_context,
            render_phase_index,
            view_submit_packet,
        )?;

        let command_buffer = &write_context.command_buffer;
        let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();

        let batch =
            &per_frame_submit_data.batched_passes.get()[batched_draw_call.batch_index as usize];

        let indirect_buffer = &per_frame_submit_data.indirect_buffer.get();

        let command_size = rafx::api::extra::indirect::indexed_indirect_command_size(
            write_context.graph_context.device_context(),
        );

        let indirect_signature = self.indirect_signature(render_phase_index);
        indirect_signature.draw_indexed_indirect(
            &*command_buffer,
            &*indirect_buffer.get_raw().buffer,
            batch.indirect_buffer_first_command_index * command_size as u32,
            batch.indirect_buffer_command_count,
        )?;

        return Ok(());
    }

    fn draw_single_render_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        render_phase_index: RenderPhaseIndex,
        submit_node_id: SubmitNodeId,
        view_submit_packet: &ViewSubmitPacket<MeshAdvRenderFeatureTypes>,
    ) -> RafxResult<()> {
        let submit_node_data = view_submit_packet
            .get_submit_node_data_from_render_phase(render_phase_index, submit_node_id)
            .as_unbatched()
            .unwrap();

        let command_buffer = &write_context.command_buffer;

        let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();

        let batch =
            &per_frame_submit_data.batched_passes.get()[submit_node_data.batch_index as usize];

        // This is equivalent code not using indirect
        /*
        let draw_data = &batch.draw_data.as_ref().unwrap()[submit_node_data.draw_data_index as usize];
        command_buffer.cmd_draw_indexed_instanced(
            draw_data.index_count,
            draw_data.index_offset,
            1,
            submit_node_data.draw_data_index,
            draw_data.vertex_offset as i32,
        );
        */

        let indirect_buffer = &per_frame_submit_data.indirect_buffer.get();
        let indirect_buffer_command_index =
            batch.indirect_buffer_first_command_index + submit_node_data.draw_data_index;

        let command_size = rafx::api::extra::indirect::indexed_indirect_command_size(
            write_context.graph_context.device_context(),
        );

        let indirect_signature = self.indirect_signature(render_phase_index);
        indirect_signature.draw_indexed_indirect(
            &*command_buffer,
            &*indirect_buffer.get_raw().buffer,
            indirect_buffer_command_index * command_size as u32,
            1,
        )?;

        Ok(())
    }
}

impl<'write> RenderFeatureWriteJob<'write> for MeshAdvWriteJob<'write> {
    fn begin_submit_node_batch(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        args: BeginSubmitNodeBatchArgs,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        let view_submit_packet = self.submit_packet.view_submit_packet(args.view_frame_index);

        if args.render_phase_index == TransparentRenderPhase::render_phase_index() {
            let batch_index = args.sort_key;

            self.setup_for_batch(
                batch_index,
                write_context,
                args.render_phase_index,
                view_submit_packet,
            )?;
        }

        Ok(())
    }

    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        args: RenderSubmitNodeArgs,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        let view_submit_packet = self.submit_packet.view_submit_packet(args.view_frame_index);

        //
        // Render nodes that do not need depth sorting will represent a batch of draws using the same pipeline/bindings.
        // Transparent nodes need to be sorted, so the render nodes in this case will represent single mesh draws.
        //
        if args.render_phase_index != TransparentRenderPhase::render_phase_index() {
            self.draw_batch(
                write_context,
                args.render_phase_index,
                args.submit_node_id,
                view_submit_packet,
            )
        } else {
            // Only transparent phase should be going through this path
            assert!(args.render_phase_index == TransparentRenderPhase::render_phase_index());
            self.draw_single_render_node(
                write_context,
                args.render_phase_index,
                args.submit_node_id,
                view_submit_packet,
            )
        }
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
