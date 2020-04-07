
use crate::slab::{RawSlabKey, SlabIndexT};
use crate::registry::RenderFeature;
use crate::registry::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use std::convert::TryInto;

static STATIC_QUAD_FEATURE_INDEX : AtomicI32 = AtomicI32::new(-1);

pub struct StaticQuadRenderFeature;

impl RenderFeature for StaticQuadRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        STATIC_QUAD_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        STATIC_QUAD_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }
}

pub struct StaticQuadRenderNode {
    // texture
    // location
}

pub struct StaticQuadRenderNodeHandle(pub RawSlabKey<StaticQuadRenderNode>);

#[derive(Copy, Clone)]
pub struct GenericRenderNodeHandle {
    render_feature_index: RenderFeatureIndex,
    slab_index: SlabIndexT
}

impl GenericRenderNodeHandle {
    pub fn render_feature_index(&self) -> RenderFeatureIndex {
        self.render_feature_index
    }

    pub fn slab_index(&self) -> SlabIndexT {
        self.slab_index
    }
}
