use crate::{FramePacket, RenderView, PrepareJob, PrepareJobSet};

pub trait ExtractJob<ExtractContextT, PrepareContextT, WriteContextT> {
    fn extract(
        self: Box<Self>,
        extract_context: &mut ExtractContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob<PrepareContextT, WriteContextT>>;

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct ExtractJobSet<ExtractContextT, PrepareContextT, WriteContextT> {
    extract_jobs: Vec<Box<dyn ExtractJob<ExtractContextT, PrepareContextT, WriteContextT>>>,
}

impl<ExtractContextT, PrepareContextT, WriteContextT> Default for ExtractJobSet<ExtractContextT, PrepareContextT, WriteContextT> {
    fn default() -> Self {
        ExtractJobSet {
            extract_jobs: Default::default(),
        }
    }
}

impl<ExtractContextT, PrepareContextT, WriteContextT> ExtractJobSet<ExtractContextT, PrepareContextT, WriteContextT> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_job(
        &mut self,
        extract_job: Box<dyn ExtractJob<ExtractContextT, PrepareContextT, WriteContextT>>,
    ) {
        self.extract_jobs.push(extract_job)
    }

    pub fn extract(
        self,
        extract_context: &mut ExtractContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> PrepareJobSet<PrepareContextT, WriteContextT> {
        log::trace!("Start extract job set");

        let mut prepare_jobs = vec![];
        for extract_job in self.extract_jobs {
            log::trace!("Start job {}", extract_job.feature_debug_name());

            let prepare_job = extract_job.extract(extract_context, frame_packet, views);
            prepare_jobs.push(prepare_job);
        }

        PrepareJobSet::new(prepare_jobs)
    }
}

use crate::RenderFeatureIndex;

use std::marker::PhantomData;
use crate::{PerFrameNode, PerViewNode};

pub trait DefaultExtractJobImpl<ExtractContextT, PrepareContextT, WriteContextT> {
    fn extract_begin(
        &mut self,
        extract_context: &mut ExtractContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    );
    fn extract_frame_node(
        &mut self,
        extract_context: &mut ExtractContextT,
        frame_node: PerFrameNode,
        frame_node_index: u32,
    );
    fn extract_view_node(
        &mut self,
        extract_context: &mut ExtractContextT,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    );
    fn extract_view_finalize(
        &mut self,
        extract_context: &mut ExtractContextT,
        view: &RenderView,
    );
    fn extract_frame_finalize(
        self,
        extract_context: &mut ExtractContextT,
    ) -> Box<dyn PrepareJob<PrepareContextT, WriteContextT>>;

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct DefaultExtractJob<ExtractContextT, PrepareContextT, WriteContextT, ExtractImplT: DefaultExtractJobImpl<ExtractContextT, PrepareContextT, WriteContextT>>
{
    extract_impl: ExtractImplT,
    phantom_data: PhantomData<(ExtractContextT, PrepareContextT, WriteContextT)>,
}

impl<ExtractContextT, PrepareContextT, WriteContextT, ExtractImplT: DefaultExtractJobImpl<ExtractContextT, PrepareContextT, WriteContextT>>
    DefaultExtractJob<ExtractContextT, PrepareContextT, WriteContextT, ExtractImplT>
{
    pub fn new(extract_impl: ExtractImplT) -> Self {
        DefaultExtractJob {
            extract_impl,
            phantom_data: Default::default(),
        }
    }
}

impl<ExtractContextT, PrepareContextT, WriteContextT, ExtractImplT: DefaultExtractJobImpl<ExtractContextT, PrepareContextT, WriteContextT>>
    ExtractJob<ExtractContextT, PrepareContextT, WriteContextT> for DefaultExtractJob<ExtractContextT, PrepareContextT, WriteContextT, ExtractImplT>
{
    fn extract(
        mut self: Box<Self>,
        extract_context: &mut ExtractContextT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob<PrepareContextT, WriteContextT>> {
        let feature_index = self.extract_impl.feature_index();

        // In the future, make features run in parallel
        log::trace!("extract_begin feature: {}", self.extract_impl.feature_debug_name());
        self.extract_impl.extract_begin(extract_context, frame_packet, views);

        // foreach frame node, call extract
        for (frame_node_index, frame_node) in
            frame_packet.frame_nodes(feature_index).iter().enumerate()
        {
            log::trace!(
                "extract_frame_node feature: {} frame node: {}",
                self.extract_impl.feature_debug_name(),
                frame_node_index
            );

            self.extract_impl
                .extract_frame_node(extract_context, *frame_node, frame_node_index as u32);
        }

        // foreach view node, call extract
        //TODO: Views can run in parallel
        for view in views {
            log::trace!(
                "extract_view_nodes feature: {} view: {}",
                self.extract_impl.feature_debug_name(),
                view.debug_name()
            );

            let view_nodes = frame_packet.view_nodes(view, feature_index);
            if let Some(view_nodes) = view_nodes {
                for (view_node_index, view_node) in view_nodes.iter().enumerate() {
                    log::trace!(
                        "extract_view_node feature: {} view node: {} node index: {}",
                        self.extract_impl.feature_debug_name(),
                        view.debug_name(),
                        view_node_index
                    );

                    self.extract_impl.extract_view_node(
                        extract_context,
                        view,
                        *view_node,
                        view_node_index as u32,
                    );
                }
            }

            // call once after all view nodes extracted
            log::trace!(
                "extract_view_finalize feature: {} view: {}",
                self.extract_impl.feature_debug_name(),
                view.debug_name()
            );
            self.extract_impl.extract_view_finalize(extract_context, view);
        }

        // call once after all nodes extracted
        log::trace!(
            "extract_frame_finalize {}",
            self.extract_impl.feature_debug_name()
        );
        self.extract_impl.extract_frame_finalize(extract_context)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.extract_impl.feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        self.extract_impl.feature_index()
    }
}
