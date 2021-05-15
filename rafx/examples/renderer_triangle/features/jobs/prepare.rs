use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::OpaqueRenderPhase;

pub struct DemoPrepareJob {}

impl DemoPrepareJob {
    pub fn new<'prepare>(
        _prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<DemoFramePacket>,
        submit_packet: Box<DemoSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(Self {}, frame_packet, submit_packet))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for DemoPrepareJob {
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

    type FramePacketDataT = DemoRenderFeatureTypes;
    type SubmitPacketDataT = DemoRenderFeatureTypes;
}
