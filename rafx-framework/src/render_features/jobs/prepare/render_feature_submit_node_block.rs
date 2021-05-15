use crate::render_features::render_features_prelude::*;
use downcast_rs::{impl_downcast, Downcast};

impl_downcast!(RenderFeatureSubmitNodeBlock);
/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`
/// to control the workload of the rendering process without identifying specific types
/// used in each `RenderFeature`'s frame packet or workload. See `SubmitNodeBlock` for
/// implementation details.
pub trait RenderFeatureSubmitNodeBlock: Downcast + Send + Sync {
    fn render_phase(&self) -> RenderPhaseIndex;

    fn num_submit_nodes(&self) -> usize;

    fn get_submit_node(
        &self,
        submit_node_id: SubmitNodeId,
    ) -> RenderFeatureSubmitNode;

    fn feature_index(&self) -> RenderFeatureIndex;
}
