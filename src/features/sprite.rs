
use crate::slab::{RawSlabKey, SlabIndexT};
use crate::registry::RenderFeature;
use crate::registry::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use crate::GenericRenderNodeHandle;
use std::convert::TryInto;


////////////////// Sprite RenderNode //////////////////

static SPRITE_FEATURE_INDEX : AtomicI32 = AtomicI32::new(-1);

pub struct SpriteRenderFeature;

impl RenderFeature for SpriteRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        SPRITE_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        SPRITE_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
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
            SpriteRenderFeature::feature_index(),
            self.0.index()
        )
    }
}
