use crate::{
    FramePacket, RenderView, PerFrameNode, PerViewNode, RenderFeatureIndex, FeatureCommandWriter,
    PreparedRenderData, FeatureSubmitNodes, MergedFrameSubmitNodes, ViewSubmitNodes,
    RenderRegistry,
};
use std::marker::PhantomData;

pub trait PrepareJob<PrepareContextT, WriteContextT> {
    fn prepare(
        self: Box<Self>,
        prepare_context: &PrepareContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (Box<dyn FeatureCommandWriter<WriteContextT>>, FeatureSubmitNodes);

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct PrepareJobSet<PrepareContextT, WriteContextT> {
    prepare_jobs: Vec<Box<dyn PrepareJob<PrepareContextT, WriteContextT>>>,
}

impl<PrepareContextT, WriteContextT> PrepareJobSet<PrepareContextT, WriteContextT> {
    pub fn new(prepare_jobs: Vec<Box<dyn PrepareJob<PrepareContextT, WriteContextT>>>) -> Self {
        PrepareJobSet { prepare_jobs }
    }

    pub fn prepare(
        self,
        prepare_context: &PrepareContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        registry: &RenderRegistry,
    ) -> Box<PreparedRenderData<WriteContextT>> {
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

        Box::new(PreparedRenderData::new(
            feature_command_writers,
            merged_submit_nodes,
        ))
    }
}

pub trait DefaultPrepareJobImpl<PrepareContextT, WriteContextT> {
    fn prepare_begin(
        &mut self,
        prepare_context: &PrepareContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        submit_nodes: &mut FeatureSubmitNodes,
    );
    fn prepare_frame_node(
        &mut self,
        prepare_context: &PrepareContextT,
        frame_node: PerFrameNode,
        frame_node_index: u32,
        submit_nodes: &mut FeatureSubmitNodes,
    );
    fn prepare_view_node(
        &mut self,
        prepare_context: &PrepareContextT,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    );
    fn prepare_view_finalize(
        &mut self,
        prepare_context: &PrepareContextT,
        view: &RenderView,
        submit_nodes: &mut ViewSubmitNodes,
    );
    fn prepare_frame_finalize(
        self,
        prepare_context: &PrepareContextT,
        submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<WriteContextT>>;

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct DefaultPrepareJob<PrepareContextT, WriteContextT, PrepareImplT: DefaultPrepareJobImpl<PrepareContextT, WriteContextT>> {
    prepare_impl: PrepareImplT,
    phantom_data: PhantomData<(PrepareContextT, WriteContextT)>,
}

impl<PrepareContextT, WriteContextT, PrepareImplT: DefaultPrepareJobImpl<PrepareContextT, WriteContextT>> DefaultPrepareJob<PrepareContextT, WriteContextT, PrepareImplT> {
    pub fn new(prepare_impl: PrepareImplT) -> Self {
        DefaultPrepareJob {
            prepare_impl,
            phantom_data: Default::default(),
        }
    }
}

impl<PrepareContextT, WriteContextT, PrepareImplT: DefaultPrepareJobImpl<PrepareContextT, WriteContextT>> PrepareJob<PrepareContextT, WriteContextT>
    for DefaultPrepareJob<PrepareContextT, WriteContextT, PrepareImplT>
{
    fn prepare(
        mut self: Box<Self>,
        prepare_context: &PrepareContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (Box<dyn FeatureCommandWriter<WriteContextT>>, FeatureSubmitNodes) {
        let feature_index = self.prepare_impl.feature_index();

        let mut submit_nodes = FeatureSubmitNodes::default();

        // In the future, make features run in parallel
        log::trace!("prepare_begin feature: {}", self.prepare_impl.feature_debug_name());
        self.prepare_impl
            .prepare_begin(prepare_context, frame_packet, views, &mut submit_nodes);


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
                prepare_context,
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
                        prepare_context,
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
                .prepare_view_finalize(prepare_context, view, &mut view_submit_nodes);

            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        // call once after all nodes extracted
        log::trace!(
            "prepare_frame_finalize {}",
            self.prepare_impl.feature_debug_name()
        );

        let writer = self.prepare_impl.prepare_frame_finalize(prepare_context, &mut submit_nodes);
        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.prepare_impl.feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        self.prepare_impl.feature_index()
    }
}
