use crate::slab::{RawSlabKey, RawSlab};
use crate::registry::RenderFeature;
use crate::registry::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use std::convert::TryInto;
use crate::{FramePacket, GenericRenderNodeHandle, RenderFeatureExtractImpl, DefaultExtractJob, ExtractJob, RenderView, PrepareJob, DefaultExtractJobImpl};
use legion::prelude::World;

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
        source: &World,
    ) {
        log::debug!("extract_begin {}", self.feature_debug_name());
    }
    fn extract_frame_node(
        &self,
        source: &World,
        entity: u32,
    ) {
        log::debug!("extract_frame_node {}", self.feature_debug_name());
    }

    fn extract_view_node(
        &self,
        source: &World,
        entity: u32,
        view: u32,
    ) {
        log::debug!("extract_view_nodes {}", self.feature_debug_name());
    }
    fn extract_view_finalize(
        &self,
        source: &World,
        view: u32,
    ) {
        log::debug!("extract_view_finalize {}", self.feature_debug_name());
    }
    fn extract_frame_finalize(
        self,
        source: &World,
    ) -> Box<PrepareJob> {
        log::debug!("extract_frame_finalize {}", self.feature_debug_name());
        Box::new(StaticQuadPrepareJob { })
    }

    fn feature_debug_name(&self) -> &'static str {
        StaticQuadRenderFeature::feature_debug_name()
    }
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
    fn extract(self: Box<Self>, source: &World, frame_packet: &FramePacket, views: &[&RenderView]) -> Box<PrepareJob> {
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

    pub fn max_node_count(&self) -> usize {
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
        self.sprites.free(&handle.0);
    }
}
