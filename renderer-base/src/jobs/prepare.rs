use crate::{FramePacket, RenderView, PerFrameNode, PerViewNode, RenderFeatureIndex, ViewSubmitNodes, MergedFrameSubmitNodes};
use crate::jobs::FeatureSubmitNodes;

pub trait PrepareJob {
    fn prepare(
        self: Box<Self>,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        submit_nodes: &mut FeatureSubmitNodes
    );

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct PrepareJobSet {
    prepare_jobs: Vec<Box<dyn PrepareJob>>,
}

impl PrepareJobSet {
    pub fn new(
        prepare_jobs: Vec<Box<dyn PrepareJob>>
    ) -> Self {
        PrepareJobSet { prepare_jobs }
    }

    pub fn prepare(
        self,
        frame_packet: &FramePacket,
        views: &[&RenderView]
    ) /*-> PrepareResult*/ {

        let mut all_submit_nodes = Vec::with_capacity(self.prepare_jobs.len());
        //let submit_handlers = Default::default();

        for prepare_job in self.prepare_jobs {
            let mut submit_nodes = FeatureSubmitNodes::new(views.len(), prepare_job.feature_index());
            prepare_job.prepare(frame_packet, views, &mut submit_nodes);
            // submit_handlers.push(result of prepare)
            all_submit_nodes.push(submit_nodes);
        }

        // TODO: Merge and return all submit nodes
        let merged_submit_nodes = MergedFrameSubmitNodes::new(all_submit_nodes, views);

        // TODO: Do we record command buffers and return them here?
    }
}




pub trait DefaultPrepareJobImpl {
    fn prepare_begin(
        &mut self,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        submit_nodes: &mut FeatureSubmitNodes
    );
    fn prepare_frame_node(
        &mut self,
        frame_node: PerFrameNode,
        frame_node_index: u32,
        submit_nodes: &mut FeatureSubmitNodes
    );
    fn prepare_view_node(
        &mut self,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes
    );
    fn prepare_view_finalize(
        &mut self,
        view: &RenderView,
        submit_nodes: &mut ViewSubmitNodes
    );
    fn prepare_frame_finalize(
        self,
        submit_nodes: &mut FeatureSubmitNodes
    );

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct DefaultPrepareJob<PrepareImplT: DefaultPrepareJobImpl> {
    prepare_impl: PrepareImplT,
}

impl<PrepareImplT: DefaultPrepareJobImpl> DefaultPrepareJob<PrepareImplT>
{
    pub fn new(prepare_impl: PrepareImplT) -> Self {
        DefaultPrepareJob {
            prepare_impl,
        }
    }
}

impl<PrepareImplT: DefaultPrepareJobImpl> PrepareJob for DefaultPrepareJob<PrepareImplT>
{
    fn prepare(
        mut self: Box<Self>,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        submit_nodes: &mut FeatureSubmitNodes
    ) {
        let feature_index = self.prepare_impl.feature_index();

        log::debug!("DefaultExtractJob::extract");

        // In the future, make features run in parallel
        log::debug!("extract_begin {}", self.prepare_impl.feature_debug_name());
        self.prepare_impl.prepare_begin(frame_packet, views, submit_nodes);

        // foreach frame node, call extract
        //for frame_node in frame_packet.fram
        log::debug!(
            "extract_frame_node {}",
            self.prepare_impl.feature_debug_name()
        );

        for (frame_node_index, frame_node) in
        frame_packet.frame_nodes(feature_index).iter().enumerate()
        {
            self.prepare_impl
                .prepare_frame_node(*frame_node, frame_node_index as u32, submit_nodes);
        }

        //TODO: Views can run in parallel
        for view in views {
            // This should be possible to adapt to run in parallel
            let mut view_submit_nodes = submit_nodes.view_submit_nodes_mut(view);

            // foreach view node, call extract
            log::debug!(
                "extract_frame_node {} {}",
                self.prepare_impl.feature_debug_name(),
                view.debug_name()
            );

            let view_nodes = frame_packet.view_nodes(view, feature_index);
            if let Some(view_nodes) = view_nodes {
                for (view_node_index, view_node) in view_nodes.iter().enumerate() {
                    self.prepare_impl.prepare_view_node(
                        view,
                        *view_node,
                        view_node_index as u32,
                        view_submit_nodes
                    );
                }
            }

            // call once after all view nodes extracted
            log::debug!(
                "extract_view_finalize {} {}",
                self.prepare_impl.feature_debug_name(),
                view.debug_name()
            );
            self.prepare_impl.prepare_view_finalize(view, view_submit_nodes);
        }

        // call once after all nodes extracted
        log::debug!(
            "extract_frame_finalize {}",
            self.prepare_impl.feature_debug_name()
        );

        self.prepare_impl.prepare_frame_finalize(submit_nodes);
    }

    fn feature_debug_name(&self) -> &'static str {
        self.prepare_impl.feature_debug_name()
    }

    fn feature_index(&self) -> u32 {
        self.prepare_impl.feature_index()
    }
}
