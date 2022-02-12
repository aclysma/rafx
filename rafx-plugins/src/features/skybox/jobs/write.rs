use rafx::render_feature_write_job_prelude::*;

use super::*;
use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{MaterialPassResource, ResourceArc, VertexDataSetLayout};
use rafx::render_features::RenderSubmitNodeArgs;
use std::marker::PhantomData;

lazy_static::lazy_static! {
    pub static ref EMPTY_VERTEX_LAYOUT : VertexDataSetLayout = {
        VertexDataSetLayout::new(vec![], RafxPrimitiveTopology::TriangleList)
    };
}

pub struct SkyboxWriteJob<'write> {
    skybox_material_pass: Option<ResourceArc<MaterialPassResource>>,
    _frame_packet: Box<SkyboxFramePacket>,
    submit_packet: Box<SkyboxSubmitPacket>,
    phantom: PhantomData<&'write ()>,
}

impl<'write> SkyboxWriteJob<'write> {
    pub fn new(
        _write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<SkyboxFramePacket>,
        submit_packet: Box<SkyboxSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        Arc::new(Self {
            skybox_material_pass: {
                frame_packet
                    .per_frame_data()
                    .get()
                    .skybox_material_pass
                    .clone()
            },
            _frame_packet: frame_packet,
            submit_packet,
            phantom: Default::default(),
        })
    }
}

impl<'write> RenderFeatureWriteJob<'write> for SkyboxWriteJob<'write> {
    fn render_submit_node(
        &self,
        write_context: &mut RenderJobCommandBufferContext,
        args: RenderSubmitNodeArgs,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_feature_debug_constants().render_submit_node);

        if let Some(skybox_material_pass) = &self.skybox_material_pass {
            let command_buffer = &write_context.command_buffer;

            let pipeline = write_context
                .resource_context()
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    Some(args.render_phase_index),
                    &skybox_material_pass,
                    &write_context.render_target_meta,
                    &EMPTY_VERTEX_LAYOUT,
                )?;

            command_buffer.cmd_bind_pipeline(&*pipeline.get_raw().pipeline)?;

            let view_submit_data = self
                .submit_packet
                .view_submit_packet(args.view_frame_index)
                .per_view_submit_data()
                .get();

            view_submit_data
                .descriptor_set_arc
                .as_ref()
                .unwrap()
                .bind(command_buffer)?;

            command_buffer.cmd_draw(3, 0)?;
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
