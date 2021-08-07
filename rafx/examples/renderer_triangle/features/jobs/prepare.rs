use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::OpaqueRenderPhase;

pub struct ExamplePrepareJob {}

impl ExamplePrepareJob {
    pub fn new<'prepare>(
        _prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<ExampleFramePacket>,
        submit_packet: Box<ExampleSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(Self {}, frame_packet, submit_packet))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for ExamplePrepareJob {
    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        if per_frame_data.triangle_material.is_some() {
            context
                .view_submit_packet()
                .push_submit_node::<OpaqueRenderPhase>((), 0, 0.);
        }
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = ExampleRenderFeatureTypes;
    type SubmitPacketDataT = ExampleRenderFeatureTypes;
}
