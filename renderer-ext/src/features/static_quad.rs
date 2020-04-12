use renderer_base::slab::{RawSlabKey, RawSlab};
use renderer_base::RenderFeature;
use renderer_base::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use std::convert::TryInto;
use renderer_base::{FramePacket, GenericRenderNodeHandle, ExtractJob, RenderView, PrepareJob};
use crate::jobs::{DefaultExtractJob, DefaultExtractJobImpl};
use legion::prelude::World;
use renderer_base::{PerFrameNode, PerViewNode};

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


struct StaticQuadExtractJobImpl {

}

impl DefaultExtractJobImpl<World> for StaticQuadExtractJobImpl {
    fn extract_begin(
        &self,
        _source: &World,
        _frame_packet: &FramePacket,
        _views: &[&RenderView]
    ) {
        log::debug!("extract_begin {}", self.feature_debug_name());
    }
    fn extract_frame_node(
        &self,
        _source: &World,
        _frame_node: PerFrameNode,
        frame_node_index: u32,
    ) {
        log::debug!("extract_frame_node {} {}", self.feature_debug_name(), frame_node_index);
    }

    fn extract_view_node(
        &self,
        _source: &World,
        _view: &RenderView,
        _view_node: PerViewNode,
        view_node_index: u32
    ) {
        log::debug!("extract_view_nodes {} {}", self.feature_debug_name(), view_node_index);
    }
    fn extract_view_finalize(
        &self,
        _source: &World,
        _view: &RenderView,
    ) {
        log::debug!("extract_view_finalize {}", self.feature_debug_name());
    }
    fn extract_frame_finalize(
        self,
        _source: &World,
    ) -> Box<dyn PrepareJob> {
        log::debug!("extract_frame_finalize {}", self.feature_debug_name());
        Box::new(StaticQuadPrepareJob { })
    }

    fn feature_debug_name(&self) -> &'static str {
        StaticQuadRenderFeature::feature_debug_name()
    }
    fn feature_index(&self) -> RenderFeatureIndex { StaticQuadRenderFeature::feature_index() }
}

pub struct StaticQuadExtractJob {
    inner: Box<DefaultExtractJob<World, StaticQuadExtractJobImpl>>
}

impl StaticQuadExtractJob {
    pub fn new() -> Self {

        let job_impl = StaticQuadExtractJobImpl {

        };

        StaticQuadExtractJob {
            inner: Box::new(DefaultExtractJob::new(job_impl))
        }
    }
}

impl ExtractJob<World> for StaticQuadExtractJob {
    fn extract(self: Box<Self>, source: &World, frame_packet: &FramePacket, views: &[&RenderView]) -> Box<dyn PrepareJob> {
        //use crate::jobs::ExtractJob;
        //self.inner.extract(frame_packet, views)
        ExtractJob::extract(self.inner, source, frame_packet, views)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.inner.feature_debug_name()
    }
}







struct StaticQuadPrepareJob {

}

impl PrepareJob for StaticQuadPrepareJob {
    fn prepare(self) {

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
    sprites: RawSlab<StaticQuadRenderNode>
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
