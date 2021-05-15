use crate::render_features::render_features_prelude::*;
use std::ops::Range;

/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`
/// to control the workload of the rendering process without identifying specific types
/// used in each `RenderFeature`'s frame packet or workload. See `PrepareJob` and the
/// `PrepareJobEntryPoints` for implementation details.
pub trait RenderFeaturePrepareJob<'prepare>: Send + Sync {
    fn begin_per_frame_prepare(&self);

    fn prepare_render_object_instance(
        &self,
        range: Range<usize>,
    );

    fn view_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewPacket;

    fn view_submit_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewSubmitPacket;

    fn prepare_render_object_instance_per_view(
        &self,
        view_packet: &dyn RenderFeatureViewPacket,
        view_submit_packet: &dyn RenderFeatureViewSubmitPacket,
        range: Range<usize>,
    );

    fn end_per_view_prepare(
        &self,
        view_packet: &dyn RenderFeatureViewPacket,
        view_submit_packet: &dyn RenderFeatureViewSubmitPacket,
    );

    fn end_per_frame_prepare(&self);

    fn num_views(&self) -> usize;

    fn num_render_object_instances(&self) -> usize;

    fn take_frame_packet(&mut self) -> Box<dyn RenderFeatureFramePacket>;

    fn take_submit_packet(&mut self) -> Box<dyn RenderFeatureSubmitPacket>;

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants;

    fn feature_index(&self) -> RenderFeatureIndex;
}
