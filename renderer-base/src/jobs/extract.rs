use crate::{FramePacket, RenderView, PrepareJob, PrepareJobSet};

pub trait ExtractJob<SourceT, WriteT> {
    fn extract(
        self: Box<Self>,
        source: &SourceT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob<WriteT>>;

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct ExtractJobSet<SourceT, WriteT> {
    extract_jobs: Vec<Box<dyn ExtractJob<SourceT, WriteT>>>,
}

impl<SourceT, WriteT> Default for ExtractJobSet<SourceT, WriteT> {
    fn default() -> Self {
        ExtractJobSet {
            extract_jobs: Default::default(),
        }
    }
}

impl<SourceT, WriteT> ExtractJobSet<SourceT, WriteT> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_job(
        &mut self,
        extract_job: Box<dyn ExtractJob<SourceT, WriteT>>,
    ) {
        self.extract_jobs.push(extract_job)
    }

    pub fn extract(
        self,
        source: &SourceT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> PrepareJobSet<WriteT> {
        log::debug!("Start extract job set");

        let mut prepare_jobs = vec![];
        for extract_job in self.extract_jobs {
            log::debug!("Start job {}", extract_job.feature_debug_name());

            let prepare_job = extract_job.extract(source, frame_packet, views);
            prepare_jobs.push(prepare_job);
        }

        PrepareJobSet::new(prepare_jobs)
    }
}

use crate::RenderFeatureIndex;

use std::marker::PhantomData;
use crate::{PerFrameNode, PerViewNode};

pub trait DefaultExtractJobImpl<SourceT, WriteT> {
    fn extract_begin(
        &mut self,
        source: &SourceT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    );
    fn extract_frame_node(
        &mut self,
        source: &SourceT,
        frame_node: PerFrameNode,
        frame_node_index: u32,
    );
    fn extract_view_node(
        &mut self,
        source: &SourceT,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    );
    fn extract_view_finalize(
        &mut self,
        source: &SourceT,
        view: &RenderView,
    );
    fn extract_frame_finalize(
        self,
        source: &SourceT,
    ) -> Box<dyn PrepareJob<WriteT>>;

    fn feature_debug_name(&self) -> &'static str;
    fn feature_index(&self) -> RenderFeatureIndex;
}

pub struct DefaultExtractJob<SourceT, WriteT, ExtractImplT: DefaultExtractJobImpl<SourceT, WriteT>>
{
    extract_impl: ExtractImplT,
    phantom_data: PhantomData<(SourceT, WriteT)>,
}

impl<SourceT, WriteT, ExtractImplT: DefaultExtractJobImpl<SourceT, WriteT>>
    DefaultExtractJob<SourceT, WriteT, ExtractImplT>
{
    pub fn new(extract_impl: ExtractImplT) -> Self {
        DefaultExtractJob {
            extract_impl,
            phantom_data: Default::default(),
        }
    }
}

impl<SourceT, WriteT, ExtractImplT: DefaultExtractJobImpl<SourceT, WriteT>>
    ExtractJob<SourceT, WriteT> for DefaultExtractJob<SourceT, WriteT, ExtractImplT>
{
    fn extract(
        mut self: Box<Self>,
        source: &SourceT,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob<WriteT>> {
        let feature_index = self.extract_impl.feature_index();

        log::debug!("DefaultExtractJob::extract");

        // In the future, make features run in parallel
        log::debug!("extract_begin {}", self.extract_impl.feature_debug_name());
        self.extract_impl.extract_begin(source, frame_packet, views);

        log::debug!(
            "extract_frame_node {}",
            self.extract_impl.feature_debug_name()
        );

        // foreach frame node, call extract
        for (frame_node_index, frame_node) in
            frame_packet.frame_nodes(feature_index).iter().enumerate()
        {
            self.extract_impl
                .extract_frame_node(source, *frame_node, frame_node_index as u32);
        }

        // foreach view node, call extract
        //TODO: Views can run in parallel
        for view in views {
            log::debug!(
                "extract_frame_node {} {}",
                self.extract_impl.feature_debug_name(),
                view.debug_name()
            );

            let view_nodes = frame_packet.view_nodes(view, feature_index);
            if let Some(view_nodes) = view_nodes {
                for (view_node_index, view_node) in view_nodes.iter().enumerate() {
                    self.extract_impl.extract_view_node(
                        source,
                        view,
                        *view_node,
                        view_node_index as u32,
                    );
                }
            }

            // call once after all view nodes extracted
            log::debug!(
                "extract_view_finalize {} {}",
                self.extract_impl.feature_debug_name(),
                view.debug_name()
            );
            self.extract_impl.extract_view_finalize(source, view);
        }

        // call once after all nodes extracted
        log::debug!(
            "extract_frame_finalize {}",
            self.extract_impl.feature_debug_name()
        );
        self.extract_impl.extract_frame_finalize(source)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.extract_impl.feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        self.extract_impl.feature_index()
    }
}
