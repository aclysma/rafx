use rafx::render_feature_write_job_prelude::*;

use super::*;
use crate::phases::WireframeRenderPhase;
use rafx::api::{RafxPrimitiveTopology, RafxVertexBufferBinding};
use rafx::framework::render_features::RenderPhase;
use rafx::framework::{MaterialPassResource, ResourceArc, VertexDataLayout, VertexDataSetLayout};
use std::marker::PhantomData;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct Debug3DVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

lazy_static::lazy_static! {
    pub static ref DEBUG_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&Debug3DVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R32G32B32A32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::LineStrip)
    };
}

pub struct Debug3DWriteJob<'write> {
    debug3d_material_pass: Option<ResourceArc<MaterialPassResource>>,
    frame_packet: Box<Debug3DFramePacket>,
    submit_packet: Box<Debug3DSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> Debug3DWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<Debug3DFramePacket>,
        submit_packet: Box<Debug3DSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            debug3d_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .debug3d_material_pass
                    .clone()
            },
            frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for Debug3DWriteJob<'write> {
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
        let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();

        if let Some(vertex_buffer) = per_frame_submit_data.vertex_buffer.as_ref() {
            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    render_phase_index,
                    self.debug3d_material_pass.as_ref().unwrap(),
                    &write_context.render_target_meta,
                    &*DEBUG_VERTEX_LAYOUT,
                )?;

            let command_buffer = &write_context.command_buffer;
            command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;

            let view_submit_packet = self.submit_packet.view_submit_packet(view_frame_index);
            if render_phase_index == WireframeRenderPhase::render_phase_index() {
                let per_view_submit_data = view_submit_packet.per_view_submit_data().get();

                per_view_submit_data
                    .descriptor_set_arc
                    .as_ref()
                    .unwrap()
                    .bind(command_buffer)?;

                command_buffer.cmd_bind_vertex_buffers(
                    0,
                    &[RafxVertexBufferBinding {
                        buffer: &*vertex_buffer.get_raw().buffer,
                        byte_offset: 0,
                    }],
                )?;
            }
        }

        Ok(())
    }

    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        _view_frame_index: ViewFrameIndex,
        _render_phase_index: RenderPhaseIndex,
        submit_node_id: SubmitNodeId,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        // The prepare phase emits a single node which will draw everything. In the future it might
        // emit a node per draw call that uses transparency
        if submit_node_id == 0 {
            let per_frame_submit_data = self.submit_packet.per_frame_submit_data().get();

            let command_buffer = &write_context.command_buffer;
            for draw_call in &per_frame_submit_data.draw_calls {
                command_buffer.cmd_draw(draw_call.count as u32, draw_call.first_element as u32)?;
            }
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
