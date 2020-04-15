use renderer_base::slab::{RawSlabKey, RawSlab};
use renderer_base::{RenderFeature, FeatureSubmitNodes};
use renderer_base::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use std::convert::TryInto;
use renderer_base::{FramePacket, GenericRenderNodeHandle, ExtractJob, RenderView, PrepareJob};
use renderer_base::{DefaultExtractJob, DefaultExtractJobImpl};
use legion::prelude::World;
use renderer_base::{PerFrameNode, PerViewNode};
use glam::Vec3;
use crate::ExtractSource;

static STATIC_QUAD_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct StaticQuadRenderFeature;

impl RenderFeature for StaticQuadRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        STATIC_QUAD_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        STATIC_QUAD_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "StaticQuadRenderFeature"
    }
}

#[derive(Default)]
struct StaticQuadExtractJobImpl {}

impl DefaultExtractJobImpl<ExtractSource> for StaticQuadExtractJobImpl {
    fn extract_begin(
        &mut self,
        _source: &ExtractSource,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
    ) {
        log::debug!("extract_begin {}", self.feature_debug_name());
    }
    fn extract_frame_node(
        &mut self,
        _source: &ExtractSource,
        _frame_node: PerFrameNode,
        frame_node_index: u32,
    ) {
        log::debug!(
            "extract_frame_node {} {}",
            self.feature_debug_name(),
            frame_node_index
        );
    }

    fn extract_view_node(
        &mut self,
        _source: &ExtractSource,
        _view: &RenderView,
        _view_node: PerViewNode,
        view_node_index: u32,
    ) {
        log::debug!(
            "extract_view_nodes {} {}",
            self.feature_debug_name(),
            view_node_index
        );
    }
    fn extract_view_finalize(
        &mut self,
        _source: &ExtractSource,
        _view: &RenderView,
    ) {
        log::debug!("extract_view_finalize {}", self.feature_debug_name());
    }
    fn extract_frame_finalize(
        self,
        _source: &ExtractSource,
    ) -> Box<dyn PrepareJob> {
        log::debug!("extract_frame_finalize {}", self.feature_debug_name());
        Box::new(StaticQuadPrepareJob {})
    }

    fn feature_debug_name(&self) -> &'static str {
        StaticQuadRenderFeature::feature_debug_name()
    }
    fn feature_index(&self) -> RenderFeatureIndex {
        StaticQuadRenderFeature::feature_index()
    }
}

pub struct StaticQuadExtractJob {
    inner: Box<DefaultExtractJob<ExtractSource, StaticQuadExtractJobImpl>>,
}

impl StaticQuadExtractJob {
    pub fn new() -> Self {
        let job_impl = StaticQuadExtractJobImpl::default();

        StaticQuadExtractJob {
            inner: Box::new(DefaultExtractJob::new(job_impl)),
        }
    }
}

impl ExtractJob<ExtractSource> for StaticQuadExtractJob {
    fn extract(
        self: Box<Self>,
        source: &ExtractSource,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> Box<dyn PrepareJob> {
        //use crate::jobs::ExtractJob;
        //self.inner.extract(frame_packet, views)
        ExtractJob::extract(self.inner, source, frame_packet, views)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.inner.feature_debug_name()
    }

    fn feature_index(&self) -> u32 {
        self.inner.feature_index()
    }
}

struct StaticQuadPrepareJob {}

impl PrepareJob for StaticQuadPrepareJob {
    fn prepare(
        self: Box<Self>,
        frame_packet: &FramePacket,
        views: &[&RenderView],
        submit_nodes: &mut FeatureSubmitNodes
    ) {

    }

    fn feature_debug_name(&self) -> &'static str {
        StaticQuadRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> u32 {
        StaticQuadRenderFeature::feature_index()
    }
}

pub struct StaticQuadRenderNode {
    // texture
    // location
}

pub struct StaticQuadRenderNodeHandle(pub RawSlabKey<StaticQuadRenderNode>);

impl Into<GenericRenderNodeHandle> for StaticQuadRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <StaticQuadRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

pub struct StaticQuadRenderNodeSet {
    sprites: RawSlab<StaticQuadRenderNode>,
}

impl StaticQuadRenderNodeSet {
    pub fn new() -> Self {
        StaticQuadRenderNodeSet {
            sprites: Default::default(),
        }
    }

    pub fn max_render_node_count(&self) -> usize {
        self.sprites.storage_size()
    }

    pub fn register_sprite(
        &mut self,
        node: StaticQuadRenderNode,
    ) -> StaticQuadRenderNodeHandle {
        StaticQuadRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn unregister_sprite(
        &mut self,
        handle: StaticQuadRenderNodeHandle,
    ) {
        self.sprites.free(handle.0);
    }
}
