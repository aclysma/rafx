use crate::Renderer;
use rafx_api::extra::upload::RafxTransferUpload;
use rafx_api::RafxResult;
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::AssetManager;
use rafx_base::resource_map::ResourceMap;
use rafx_framework::render_features::render_features_prelude::*;
use rafx_framework::RenderResources;
use std::path::PathBuf;
use std::sync::Arc;

/// A `RenderFeaturePlugin` defines a `RenderFeature` for the `Renderer`. The `RenderFeaturePlugin`
/// completely encapsulates the logic needed by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`.
///
/// # Initialization
///
/// `configure_render_registry` and `initialize_static_resources` setup the `RenderFeature` for usage with
/// the `Renderer`.
///
/// # Extract
///
/// `is_relevant`, `is_view_relevant`, and `requires_visible_render_objects` are used to identify which
/// `RenderView`s need to be included in the `RenderFeature`'s frame packet. Then, `calculate_frame_packet_size`
/// is used to size the `FramePacket`, `new_frame_packet` allocates the memory for the `FramePacket`, and
/// `populate_frame_packet` fills the `FramePacket` with the mapping of `RenderObjectInstance` and
/// `RenderObjectInstancePerView` for each `ViewPacket`. `new_extract_job` wraps the `FramePacket` to
/// return a `RenderFeatureExtractJob`.
///
/// # Prepare
///
/// `new_submit_packet` sizes and allocates the memory for a `SubmitPacket` using the `FramePacket` as
/// a reference. `new_prepare_job` wraps the `FramePacket` and `SubmitPacket` to return a `RenderFeaturePrepareJob`.
///
/// # Write
///
/// `new_write_job` wraps the `FramePacket` and `SubmitPacket` to return a `RenderFeatureWriteJob`.
pub trait RenderFeaturePlugin: Send + Sync {
    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants;
    fn feature_index(&self) -> RenderFeatureIndex;

    /// Returns `true` if the `RenderView` represented by the `RenderViewVisibilityQuery` should be
    /// included in the `FramePacket` for this `RenderFeature`. Most features should only need to
    /// implement `is_view_relevant` and `requires_visible_render_objects`.
    fn is_relevant(
        &self,
        view_visibility: &RenderViewVisibilityQuery,
    ) -> bool {
        let view = &view_visibility.view;
        if !view.feature_index_is_relevant(self.feature_index()) {
            return false;
        }

        if self.is_view_relevant(&view) {
            return if self.requires_visible_render_objects() {
                view_visibility
                    .render_object_instances_per_view(self.feature_index())
                    .is_some()
            } else {
                true
            };
        }

        false
    }

    /// Returns `true` if the `RenderView` should be included in the `FramePacket` for this `RenderFeature`.
    /// This is normally implemented by checking if the `RenderView`'s `RenderPhaseMask` includes the
    /// `RenderPhase`s needed by the `RenderFeature`.
    fn is_view_relevant(
        &self,
        view: &RenderView,
    ) -> bool;

    /// Returns `true` if this `RenderFeature` requires at least one `RenderObject` associated with this
    /// `RenderFeature` in the `RenderViewVisibilityQuery` in order to include the `RenderView` in the
    /// `FramePacket` for this `RenderFeature`. This is normally `true` if the `RenderFeature` defines
    /// a `RenderObjectSet` and `false` otherwise.
    fn requires_visible_render_objects(&self) -> bool;

    fn add_asset_paths(
        &self,
        _asset_paths: &mut Vec<PathBuf>,
    ) {
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry
    }

    fn initialize_static_resources(
        &self,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        _render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn add_render_views(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        _render_view_set: &RenderViewSet,
        _render_views: &mut Vec<RenderView>,
    ) {
    }

    /// Determines the unique set of `RenderObjectInstance`s in `FramePacket` and the size of each
    /// `ViewPacket` in the `FramePacket`.
    fn calculate_frame_packet_size<'extract>(
        &self,
        _extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
        render_object_instance_object_ids: &mut RenderObjectInstanceObjectIds,
        frame_packet_size: &mut FramePacketSize,
    ) {
        Renderer::calculate_frame_packet_size(
            self.feature_debug_constants(),
            self.feature_index(),
            |view_visibility| self.is_relevant(view_visibility),
            visibility_results,
            render_object_instance_object_ids,
            frame_packet_size,
        );
    }

    /// Allocates the memory for the `FramePacket`.
    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket>;

    /// Creates the mapping of `RenderObjectInstance` and `RenderObjectInstancePerView` in the
    /// `FramePacket`. This is done separately from `new_frame_packet` so that all of the frame
    /// packets can be allocated at once and then populated in parallel.
    fn populate_frame_packet<'extract>(
        &self,
        _extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
        _frame_packet_size: &FramePacketSize,
        frame_packet: &mut Box<dyn RenderFeatureFramePacket>,
    ) {
        Renderer::populate_frame_packet(
            self.feature_debug_constants(),
            self.feature_index(),
            |view_visibility| self.is_relevant(view_visibility),
            visibility_results,
            frame_packet,
        );
    }

    /// Returns a `RenderFeatureExtractJob` wrapping the `FramePacket`.
    fn new_extract_job<'extract>(
        &self,
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>;

    /// Determines the size of the `SubmitPacket` according to the size of the `FramePacket` and
    /// allocates the memory for it. The `SubmitPacket` is populated by the `RenderFeaturePrepareJob`
    /// so there is no equivalent to `populate_frame_packet`.
    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket>;

    /// Returns a `RenderFeaturePrepareJob` wrapping the `FramePacket` and `SubmitPacket`.
    fn new_prepare_job<'prepare>(
        &self,
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
        submit_packet: Box<dyn RenderFeatureSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>;

    /// Returns a `RenderFeatureWriteJob` wrapping the `FramePacket` and `SubmitPacket`.
    fn new_write_job<'write>(
        &self,
        write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
        submit_packet: Box<dyn RenderFeatureSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write>;
}
