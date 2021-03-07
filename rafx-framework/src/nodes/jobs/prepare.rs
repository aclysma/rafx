use crate::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, MergedFrameSubmitNodes,
    PreparedRenderData, RenderFeatureIndex, RenderJobPrepareContext, RenderRegistry, RenderView,
};

pub trait PrepareJob: Send {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes);

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct PrepareJobSet {
    prepare_jobs: Vec<Box<dyn PrepareJob>>,
}

impl PrepareJobSet {
    pub fn new(prepare_jobs: Vec<Box<dyn PrepareJob>>) -> Self {
        PrepareJobSet { prepare_jobs }
    }

    pub fn prepare(
        self,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[RenderView],
        registry: &RenderRegistry,
    ) -> PreparedRenderData {
        let mut feature_command_writers = Vec::with_capacity(self.prepare_jobs.len());
        let mut all_submit_nodes = Vec::with_capacity(self.prepare_jobs.len());

        //TODO: Kick these to happen in parallel
        for prepare_job in self.prepare_jobs {
            let (writer, submit_nodes) = prepare_job.prepare(prepare_context, frame_packet, views);

            feature_command_writers.push(writer);
            all_submit_nodes.push(submit_nodes);
        }

        // Merge all submit nodes
        let merged_submit_nodes = MergedFrameSubmitNodes::new(all_submit_nodes, registry);

        PreparedRenderData::new(feature_command_writers, merged_submit_nodes)
    }
}
