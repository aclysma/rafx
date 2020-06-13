use renderer_base::{RenderFeature, RenderFeatureIndex, DefaultExtractJob, ExtractJob, GenericRenderNodeHandle, RenderNodeSet, RenderNodeCount};
use std::sync::atomic::{Ordering, AtomicI32};
use glam::f32::Vec3;
use crate::{ExtractSource, CommandWriterContext};
use legion::prelude::Entity;
use renderer_base::slab::{RawSlabKey, RawSlab};
use std::convert::TryInto;

mod extract;
use extract::SpriteExtractJobImpl;

mod prepare;
use prepare::SpritePrepareJobImpl;

mod write;
use write::SpriteCommandWriter;

pub fn create_sprite_extract_job() -> Box<dyn ExtractJob<ExtractSource, CommandWriterContext>> {
    Box::new(DefaultExtractJob::new(SpriteExtractJobImpl::default()))
}

//
// This is boiler-platish
//
pub struct SpriteRenderNode {
    pub entity: Entity, // texture
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

#[derive(Default)]
pub struct SpriteRenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>,
}

impl SpriteRenderNodeSet {
    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn register_sprite_with_handle<F: FnMut(SpriteRenderNodeHandle) -> SpriteRenderNode>(
        &mut self,
        mut f: F,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(
            self.sprites
                .allocate_with_key(|handle| (f)(SpriteRenderNodeHandle(handle))),
        )
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

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.sprites.storage_size() as RenderNodeCount
    }
}

//
// This is boilerplate that could be macro'd
//
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

#[derive(Debug, Clone)]
pub(self) struct ExtractedSpriteData {
    position: Vec3,
    alpha: f32,
}