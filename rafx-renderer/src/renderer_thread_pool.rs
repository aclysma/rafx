use crate::RenderFeaturePlugin;
use rafx_framework::render_features::render_features_prelude::*;
use std::sync::Arc;

/// An application may implement `RendererThreadPool` to control the degree and method
/// of parallelization used for each entry point defined by `RendererThreadPool`.
///
/// # Extract
///
/// The `extract` step contains the `run_view_visibility_jobs`, `count_render_features_render_objects`,
/// `create_extract_jobs`, and `run_extract_jobs` entry points.
///
/// # Prepare
///
/// The `prepare` step contains the `run_prepare_jobs` and `create_submit_node_blocks` entry points.
///
/// # Write
///
/// The `write` step is not able to be parallelized in this release.
pub trait RendererThreadPool: Sync + Send {
    /// Each `RenderView` has an associated `ViewVisibilityJob` for calculating visible render objects
    /// from that `RenderView`.
    fn run_view_visibility_jobs<'extract>(
        &mut self,
        view_visibility_jobs: &[Arc<ViewVisibilityJob>],
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_resource: &VisibilityResource,
    ) -> Vec<RenderViewVisibilityQuery>;

    /// All of the visibility results from `run_view_visibility_jobs` for all of the `RenderView`s
    /// must be processed to size the `FramePacket` for each `RenderFeature`.
    fn count_render_features_render_objects<'extract>(
        &mut self,
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: &Vec<RenderViewVisibilityQuery>,
    );

    /// Each `RenderFeature` must populate its `FramePacket` and `ViewPacket`s with the mapping of
    /// `RenderObjectInstance` and `RenderObjectInstancePerView` for the current frame using the
    /// visibility results.
    fn create_extract_jobs<'extract>(
        &mut self,
        features: &Vec<Arc<dyn RenderFeaturePlugin>>,
        extract_context: &RenderJobExtractContext<'extract>,
        visibility_results: Vec<RenderViewVisibilityQuery>,
    ) -> Vec<Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>>;

    /// Each `RenderFeature` uses the `RenderFeatureExtractJob` to copy data from the game world and
    /// other resources into the `RenderFeature`s `FramePacket`.
    fn run_extract_jobs<'extract>(
        &mut self,
        extract_jobs: &Vec<Arc<dyn RenderFeatureExtractJob<'extract> + 'extract>>,
        visibility_resource: &VisibilityResource,
    );

    /// Each `RenderFeature` uses the `RenderFeaturePrepareJob` to process data from the `FramePacket`
    /// into the `RenderFeature`s `SubmitPacket`.
    fn run_prepare_jobs<'prepare>(
        &mut self,
        prepare_jobs: &Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>,
    );

    /// The output of each `RenderFeature`'s `prepare` step is one or more `SubmitNodeBlock`s containing
    /// `SubmitNode`s for a particular `View` and `RenderPhase`. This entry point is responsible for
    /// combining all `SubmitBlock`s across **all** `RenderFeature`s matching a specific `View` and
    /// `RenderPhase` into a single contiguous `ViewPhaseSubmitNodeBlock` and then sorting all of the
    /// `SubmitNode`s in that block according to the `RenderPhase`s sort function.
    fn create_submit_node_blocks<'prepare>(
        &mut self,
        render_registry: &RenderRegistry,
        render_view_submit_nodes: &RenderViewSubmitNodeCount,
        finished_prepare_jobs: &Vec<Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare>>,
    ) -> SubmitNodeBlocks;

    fn clone_to_box(&mut self) -> Box<dyn RendererThreadPool>;
}
