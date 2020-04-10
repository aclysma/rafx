
use crate::slab::{RawSlabKey, SlabIndexT};
use crate::registry::RenderFeature;
use crate::registry::RenderFeatureImpl;
use crate::registry::RenderFeatureIndex;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicI32;
use crate::{GenericRenderNodeHandle, FramePacket};
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

    fn create_render_feature_impl() -> Box<RenderFeatureImpl> {
        Box::new(Self)
    }
}

impl RenderFeatureImpl for SpriteRenderFeature {
    fn feature_index(&self) -> RenderFeatureIndex { <Self as RenderFeature>::feature_index() }

    fn extract_begin(&self, frame_packet: &FramePacket) { println!("extract_begin {}", core::any::type_name::<Self>()); }
    fn extract_frame_node(&self, frame_packet: &FramePacket) { println!("extract_frame_node {}", core::any::type_name::<Self>()); }
    fn extract_view_nodes(&self, frame_packet: &FramePacket) { println!("extract_view_nodes {}", core::any::type_name::<Self>()); }
    fn extract_view_finalize(&self, frame_packet: &FramePacket) { println!("extract_view_finalize {}", core::any::type_name::<Self>()); }
    fn extract_frame_finalize(&self, frame_packet: &FramePacket) { println!("extract_frame_finalize {}", core::any::type_name::<Self>()); }
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
            self.0.index()
        )
    }
}
