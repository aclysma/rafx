use crate::render_features::render_features_prelude::*;
use std::ops::Range;

/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`
/// to control the workload of the rendering process without identifying specific types
/// used in each `RenderFeature`'s frame packet or workload. See `ExtractJob` and the
/// `ExtractJobEntryPoints` for implementation details.
pub trait RenderFeatureExtractJob<'extract>: Send + Sync {
    fn begin_per_frame_extract(&self);

    fn extract_render_object_instance(
        &self,
        visibility_resource: &VisibilityResource,
        range: Range<usize>,
    );

    fn view_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewPacket;

    fn extract_render_object_instance_per_view(
        &self,
        view_packet: &dyn RenderFeatureViewPacket,
        visibility_resource: &VisibilityResource,
        range: Range<usize>,
    );

    fn end_per_view_extract(
        &self,
        view_packet: &dyn RenderFeatureViewPacket,
    );

    fn end_per_frame_extract(&self);

    fn num_views(&self) -> usize;

    fn num_render_object_instances(&self) -> usize;

    fn take_frame_packet(&mut self) -> Box<dyn RenderFeatureFramePacket>;

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants;

    fn feature_index(&self) -> RenderFeatureIndex;
}
