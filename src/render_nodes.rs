//
// use crate::slab::{RawSlabKey, SlabIndexT};
// use crate::registry::RenderFeature;
// use crate::registry::RenderFeatureIndex;
// use std::sync::atomic::Ordering;
// use std::sync::atomic::AtomicI32;
//
//
// ////////////////// StaticQuad RenderNode //////////////////
// pub struct StaticQuadRenderNode {
//     // texture
//     // location
// }
//
// pub struct StaticQuadRenderNodeHandle(pub RawSlabKey<StaticQuadRenderNode>);
//
// #[derive(Copy, Clone)]
// pub struct GenericRenderNodeHandle {
//     render_feature_index: RenderFeatureIndex,
//     slab_index: SlabIndexT
// }
//
// impl GenericRenderNodeHandle {
//     pub fn new(
//         render_feature_index: RenderFeatureIndex,
//         slab_index: SlabIndexT
//     ) -> Self {
//         GenericRenderNodeHandle {
//             render_feature_index,
//             slab_index
//         }
//     }
//
//     pub fn render_feature_index(&self) -> RenderFeatureIndex {
//         self.render_feature_index
//     }
//
//     pub fn slab_index(&self) -> SlabIndexT {
//         self.slab_index
//     }
// }
