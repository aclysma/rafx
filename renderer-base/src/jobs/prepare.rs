use crate::{
    FramePacket, RenderView, PerFrameNode, PerViewNode, RenderFeatureIndex, FeatureCommandWriter,
    PreparedRenderData, FeatureSubmitNodes, MergedFrameSubmitNodes, ViewSubmitNodes,
    RenderRegistry,
};
use std::marker::PhantomData;

pub trait PrepareJob<WriteT> {
    fn prepare(
        self: Box<Self>,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (Box<dyn FeatureCommandWriter<WriteT>>, FeatureSubmitNodes);

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct PrepareJobSet<WriteT> {
    prepare_jobs: Vec<Box<dyn PrepareJob<WriteT>>>,
}

impl<WriteT> PrepareJobSet<WriteT> {
    pub fn new(prepare_jobs: Vec<Box<dyn PrepareJob<WriteT>>>) -> Self {
        PrepareJobSet { prepare_jobs }
    }

    pub fn prepare(
        self,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        registry: &RenderRegistry,
    ) -> Box<PreparedRenderData<WriteT>> {
        let mut feature_command_writers = Vec::with_capacity(self.prepare_jobs.len());
        let mut all_submit_nodes = Vec::with_capacity(self.prepare_jobs.len());

        //TODO: Kick these to happen in parallel
        for prepare_job in self.prepare_jobs {
            let (writer, submit_nodes) = prepare_job.prepare(frame_packet, views);

            feature_command_writers.push(writer);
            all_submit_nodes.push(submit_nodes);
        }

        // Merge all submit nodes
        let merged_submit_nodes = MergedFrameSubmitNodes::new(all_submit_nodes, registry);

        Box::new(PreparedRenderData::new(
            feature_command_writers,
            merged_submit_nodes,
        ))
    }
}

pub trait DefaultPrepareJobImpl<WriteT> {
    fn prepare_begin(
        &mut self,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        submit_nodes: &mut FeatureSubmitNodes,
    );
    fn prepare_frame_node(
        &mut self,
        frame_node: PerFrameNode,
        frame_node_index: u32,
        submit_nodes: &mut FeatureSubmitNodes,
    );
    fn prepare_view_node(
        &mut self,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    );
    fn prepare_view_finalize(
        &mut self,
        view: &RenderView,
        submit_nodes: &mut ViewSubmitNodes,
    );
    fn prepare_frame_finalize(
        self,
        submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<WriteT>>;

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct DefaultPrepareJob<WriteT, PrepareImplT: DefaultPrepareJobImpl<WriteT>> {
    prepare_impl: PrepareImplT,
    phantom_data: PhantomData<WriteT>,
}

impl<WriteT, PrepareImplT: DefaultPrepareJobImpl<WriteT>> DefaultPrepareJob<WriteT, PrepareImplT> {
    pub fn new(prepare_impl: PrepareImplT) -> Self {
        DefaultPrepareJob {
            prepare_impl,
            phantom_data: Default::default(),
        }
    }
}

impl<WriteT, PrepareImplT: DefaultPrepareJobImpl<WriteT>> PrepareJob<WriteT>
    for DefaultPrepareJob<WriteT, PrepareImplT>
{
    fn prepare(
        mut self: Box<Self>,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (Box<dyn FeatureCommandWriter<WriteT>>, FeatureSubmitNodes) {
        let feature_index = self.prepare_impl.feature_index();

        let mut submit_nodes = FeatureSubmitNodes::default();

        // In the future, make features run in parallel
        log::trace!("prepare_begin feature: {}", self.prepare_impl.feature_debug_name());
        self.prepare_impl
            .prepare_begin(frame_packet, views, &mut submit_nodes);


        // foreach frame node, call extract
        for (frame_node_index, frame_node) in
            frame_packet.frame_nodes(feature_index).iter().enumerate()
        {
            log::trace!(
                "prepare_frame_node feature: {} frame node: {}",
                self.prepare_impl.feature_debug_name(),
                frame_node_index
            );

            self.prepare_impl.prepare_frame_node(
                *frame_node,
                frame_node_index as u32,
                &mut submit_nodes,
            );
        }

        // foreach view node, call extract
        //TODO: Views can run in parallel
        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.prepare_impl.feature_index(), view.render_phase_mask());

            // foreach view node, call extract
            log::trace!(
                "prepare_view_nodes feature: {} view: {}",
                self.prepare_impl.feature_debug_name(),
                view.debug_name()
            );

            let view_nodes = frame_packet.view_nodes(view, feature_index);
            if let Some(view_nodes) = view_nodes {
                for (view_node_index, view_node) in view_nodes.iter().enumerate() {
                    log::trace!(
                        "prepare_view_node feature: {} view: {} node index: {}",
                        self.prepare_impl.feature_debug_name(),
                        view.debug_name(),
                        view_node_index
                    );

                    self.prepare_impl.prepare_view_node(
                        view,
                        *view_node,
                        view_node_index as u32,
                        &mut view_submit_nodes,
                    );
                }
            }

            // call once after all view nodes extracted
            log::trace!(
                "prepare_view_finalize feature: {} view: {}",
                self.prepare_impl.feature_debug_name(),
                view.debug_name()
            );

            self.prepare_impl
                .prepare_view_finalize(view, &mut view_submit_nodes);

            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        // call once after all nodes extracted
        log::trace!(
            "prepare_frame_finalize {}",
            self.prepare_impl.feature_debug_name()
        );

        let writer = self.prepare_impl.prepare_frame_finalize(&mut submit_nodes);
        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.prepare_impl.feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        self.prepare_impl.feature_index()
    }
}
