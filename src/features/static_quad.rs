
use crate::slab::{RawSlabKey, SlabIndexT};
use crate::registry::{RenderFeature, RenderFeatureImpl};
use crate::registry::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use std::convert::TryInto;
use crate::FramePacket;

static STATIC_QUAD_FEATURE_INDEX : AtomicI32 = AtomicI32::new(-1);

pub struct StaticQuadRenderFeature;

impl RenderFeature for StaticQuadRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        STATIC_QUAD_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        STATIC_QUAD_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn create_render_feature_impl() -> Box<RenderFeatureImpl> {
        Box::new(Self)
    }
}

impl RenderFeatureImpl for StaticQuadRenderFeature {
    fn feature_index(&self) -> RenderFeatureIndex { <Self as RenderFeature>::feature_index() }

    fn extract_begin(&self, frame_packet: &FramePacket) { println!("extract_begin {}", core::any::type_name::<Self>()); }
    fn extract_frame_node(&self, frame_packet: &FramePacket) { println!("extract_frame_node {}", core::any::type_name::<Self>()); }
    fn extract_view_nodes(&self, frame_packet: &FramePacket) { println!("extract_view_nodes {}", core::any::type_name::<Self>()); }
    fn extract_view_finalize(&self, frame_packet: &FramePacket) { println!("extract_view_finalize {}", core::any::type_name::<Self>()); }
    fn extract_frame_finalize(&self, frame_packet: &FramePacket) { println!("extract_frame_finalize {}", core::any::type_name::<Self>()); }
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
