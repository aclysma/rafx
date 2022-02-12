use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::DebugPipRenderPhase;

pub struct DebugPipPrepareJob {}

impl DebugPipPrepareJob {
    pub fn new<'prepare>(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<DebugPipFramePacket>,
        submit_packet: Box<DebugPipSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self {},
            prepare_context,
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for DebugPipPrepareJob {
    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();

        if let Some(_) = &per_frame_data.debug_pip_material_pass {
            context
                .view_submit_packet()
                .push_submit_node::<DebugPipRenderPhase>((), 0, 0.);
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

    type FramePacketDataT = DebugPipRenderFeatureTypes;
    type SubmitPacketDataT = DebugPipRenderFeatureTypes;
}
