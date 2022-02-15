use rafx::render_feature_write_job_prelude::*;

use super::*;
use rafx::api::{
    RafxIndexBufferBinding, RafxIndexType, RafxPrimitiveTopology, RafxVertexAttributeRate,
    RafxVertexBufferBinding,
};
use rafx::framework::{MaterialPassResource, ResourceArc, VertexDataLayout, VertexDataSetLayout};
use rafx::render_features::{BeginSubmitNodeBatchArgs, RenderSubmitNodeArgs};
use std::marker::PhantomData;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct TileLayerVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

lazy_static::lazy_static! {
    pub static ref TILE_LAYER_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;
        VertexDataLayout::build_vertex_layout(&TileLayerVertex::default(), RafxVertexAttributeRate::Vertex, |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

pub struct TileLayerWriteJob<'write> {
    tile_layer_material_pass: Option<ResourceArc<MaterialPassResource>>,
    render_objects: TileLayerRenderObjectSet,
    _frame_packet: Box<TileLayerFramePacket>,
    submit_packet: Box<TileLayerSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> TileLayerWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<TileLayerFramePacket>,
        submit_packet: Box<TileLayerSubmitPacket>,
        render_objects: TileLayerRenderObjectSet,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            tile_layer_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .tile_layer_material_pass
                    .clone()
            },
            render_objects,
            _frame_packet: frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for TileLayerWriteJob<'write> {
    fn begin_submit_node_batch(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        args: BeginSubmitNodeBatchArgs,
    ) -> RafxResult<()> {
        if !args.feature_changed {
            return Ok(());
        }

        profiling::scope!(super::render_feature_debug_constants().begin_submit_node_batch);

        let command_buffer = &write_context.command_buffer;

        let pipeline = write_context
            .resource_context()
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                Some(args.render_phase_index),
                self.tile_layer_material_pass.as_ref().unwrap(),
                &write_context.render_target_meta,
                &TILE_LAYER_VERTEX_LAYOUT,
            )
            .unwrap();

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        Ok(())
    }

    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        args: RenderSubmitNodeArgs,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        let command_buffer = &write_context.command_buffer;

        let view_submit_packet = self.submit_packet.view_submit_packet(args.view_frame_index);

        let per_view_submit_data = view_submit_packet.per_view_submit_data().get();

        // Bind per-pass data (UBO with view/proj matrix, sampler)
        per_view_submit_data
            .descriptor_set_arc
            .as_ref()
            .unwrap()
            .bind(command_buffer)?;

        let submit_node = view_submit_packet
            .get_submit_node_data_from_render_phase(args.render_phase_index, args.submit_node_id);

        let render_objects = self.render_objects.read();
        let tile_layer_render_object = render_objects.get_id(&submit_node.render_object_id);
        tile_layer_render_object
            .per_layer_descriptor_set
            .bind(command_buffer)?;

        for draw_call in &tile_layer_render_object.draw_call_data {
            command_buffer.cmd_bind_vertex_buffers(
                0,
                &[RafxVertexBufferBinding {
                    buffer: &tile_layer_render_object.vertex_buffer.get_raw().buffer,
                    byte_offset: draw_call.vertex_data_offset_in_bytes as u64,
                }],
            )?;

            command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
                buffer: &tile_layer_render_object.index_buffer.get_raw().buffer,
                byte_offset: draw_call.index_data_offset_in_bytes as u64,
                index_type: RafxIndexType::Uint16,
            })?;

            command_buffer.cmd_draw_indexed(draw_call.index_count, 0, 0)?;
        }

        Ok(())
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
