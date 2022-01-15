use rafx::render_feature_write_job_prelude::*;

use super::*;
use rafx::api::RafxPrimitiveTopology;
use rafx::api::{
    RafxIndexBufferBinding, RafxIndexType, RafxVertexAttributeRate, RafxVertexBufferBinding,
};
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};
use std::marker::PhantomData;
use std::sync::atomic::Ordering;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct SpriteVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
    pub color: [u8; 4],
}

lazy_static::lazy_static! {
    pub static ref SPRITE_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&SpriteVertex::default(), RafxVertexAttributeRate::Vertex,  |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.tex_coord, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R8G8B8A8_UNORM);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

pub struct SpriteWriteJob<'write> {
    sprite_material_pass: Option<ResourceArc<MaterialPassResource>>,
    frame_packet: Box<SpriteFramePacket>,
    submit_packet: Box<SpriteSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> SpriteWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<SpriteFramePacket>,
        submit_packet: Box<SpriteSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            sprite_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .sprite_material_pass
                    .clone()
            },
            frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for SpriteWriteJob<'write> {
    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex {
        self.frame_packet.view_frame_index(view)
    }

    fn apply_setup(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        view_frame_index: ViewFrameIndex,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().apply_setup);

        let command_buffer = &write_context.command_buffer;

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                Some(render_phase_index),
                self.sprite_material_pass.as_ref().unwrap(),
                &write_context.render_target_meta,
                &SPRITE_VERTEX_LAYOUT,
            )
            .unwrap();

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        // Bind per-pass data (UBO with view/proj matrix, sampler)
        let view_submit_packet = self.submit_packet.view_submit_packet(view_frame_index);
        let per_view_submit_data = view_submit_packet.per_view_submit_data().get();

        per_view_submit_data
            .descriptor_set_arc
            .as_ref()
            .unwrap()
            .bind(command_buffer)?;

        command_buffer.cmd_bind_vertex_buffers(
            0,
            &[RafxVertexBufferBinding {
                buffer: &per_view_submit_data
                    .vertex_buffer
                    .as_ref()
                    .unwrap()
                    .get_raw()
                    .buffer,
                byte_offset: 0,
            }],
        )?;

        command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
            buffer: &per_view_submit_data
                .index_buffer
                .as_ref()
                .unwrap()
                .get_raw()
                .buffer,
            byte_offset: 0,
            index_type: RafxIndexType::Uint16,
        })?;

        Ok(())
    }

    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        view_frame_index: ViewFrameIndex,
        render_phase_index: RenderPhaseIndex,
        submit_node_id: SubmitNodeId,
    ) -> RafxResult<()> {
        let command_buffer = &write_context.command_buffer;

        let view_submit_packet = self.submit_packet.view_submit_packet(view_frame_index);
        let submit_node = &view_submit_packet
            .get_submit_node_data_from_render_phase(render_phase_index, submit_node_id);

        // Bind per-draw-call data (i.e. texture)
        submit_node
            .texture_descriptor_set
            .as_ref()
            .unwrap()
            .bind(command_buffer)?;

        command_buffer.cmd_draw_indexed(
            submit_node.index_count.load(Ordering::Relaxed),
            submit_node.index_data_offset_index,
            submit_node.vertex_data_offset_index as i32,
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
