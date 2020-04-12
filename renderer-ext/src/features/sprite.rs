use renderer_base::slab::{RawSlabKey, RawSlab};
use renderer_base::RenderFeature;
use renderer_base::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use renderer_base::{FramePacket, GenericRenderNodeHandle, ExtractJob, PrepareJob, RenderView, RenderNodeSet};
use crate::jobs::{DefaultExtractJob, DefaultExtractJobImpl};
use std::convert::TryInto;
use legion::prelude::World;
use renderer_base::{PerFrameNode, PerViewNode};

static SPRITE_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct SpriteRenderFeature;

impl RenderFeature for SpriteRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        SPRITE_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        SPRITE_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "SpriteRenderFeature"
    }
}






struct SpriteExtractJobImpl {

}

impl DefaultExtractJobImpl<World> for SpriteExtractJobImpl {
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
        Box::new(SpritePrepareJob { })
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }
    fn feature_index(&self) -> RenderFeatureIndex { SpriteRenderFeature::feature_index() }
}

pub struct SpriteExtractJob {
    inner: Box<DefaultExtractJob<World, SpriteExtractJobImpl>>
}

impl SpriteExtractJob {
    pub fn new() -> Self {
        let job_impl = SpriteExtractJobImpl {

        };

        SpriteExtractJob {
            inner: Box::new(DefaultExtractJob::new(job_impl))
        }
    }
}

impl ExtractJob<World> for SpriteExtractJob {
    fn extract(self: Box<Self>, source: &World, frame_packet: &FramePacket, views: &[&RenderView]) -> Box<dyn PrepareJob> {
        self.inner.extract(source, frame_packet, views)
    }

    fn feature_debug_name(&self) -> &'static str {
        self.inner.feature_debug_name()
    }
}

struct SpritePrepareJob {

}

impl PrepareJob for SpritePrepareJob {
    fn prepare(self) {

    }
}


pub struct SpriteRenderNode {
    // entity handle
    // texture
}

#[derive(Copy, Clone)]
pub struct SpriteRenderNodeHandle(pub RawSlabKey<SpriteRenderNode>);

impl Into<GenericRenderNodeHandle> for SpriteRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <SpriteRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}




pub struct SpriteRenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>
}

impl SpriteRenderNodeSet {
    pub fn new() -> Self {
        SpriteRenderNodeSet {
            sprites: Default::default(),
        }
    }

    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn unregister_sprite(
        &mut self,
        handle: SpriteRenderNodeHandle,
    ) {
        self.sprites.free(handle.0);
    }
}

impl RenderNodeSet for SpriteRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }

    fn max_render_node_count(&self) -> usize {
        self.sprites.storage_size()
    }
}