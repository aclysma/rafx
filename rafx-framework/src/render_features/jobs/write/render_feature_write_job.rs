use crate::render_features::{
    RenderFeatureDebugConstants, RenderFeatureIndex, RenderJobBeginExecuteGraphContext,
    RenderJobCommandBufferContext, RenderPhaseIndex, RenderView, SubmitNodeId, ViewFrameIndex,
};
use rafx_api::RafxResult;

/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`
/// to control the workload of the rendering process without identifying specific types
/// used in each `RenderFeature`'s frame packet or workload.
pub trait RenderFeatureWriteJob<'write>: Sync + Send {
    /// Called once at the start of executing the `PreparedRenderData` in `PreparedRenderGraph::execute_graph`.
    /// This can be used to start GPU transfers or other work prior to drawing any submit nodes related to
    /// this `RenderFeature`.
    fn on_begin_execute_graph(
        &self,
        _begin_execute_graph_context: &mut RenderJobBeginExecuteGraphContext,
    ) -> RafxResult<()> {
        Ok(())
    }

    /// Returns the `ViewFrameIndex` of a `RenderView` in the `RenderFeature`'s frame packet. This
    /// function **must** panic if the `RenderView` is not part of the `RenderFeature`'s frame packet.
    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex;

    /// Called by `PreparedRenderData` in `write_view_phase` whenever the current `RenderFeatureIndex`
    /// changes **into** this `RenderFeature`. This can be used to setup pipelines or other expensive state
    /// changes that can remain constant for subsequent `render_submit_node` calls.
    fn apply_setup(
        &self,
        _write_context: &mut RenderJobCommandBufferContext,
        _view_frame_index: ViewFrameIndex,
        _render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        Ok(())
    }

    /// Called by `PreparedRenderData` in `write_view_phase` for each `RenderFeatureSubmitNode` associated
    /// with this `RenderFeature`. This is normally where the actual GPU draw commands are implemented for
    /// the `RenderFeature`. Each call to `render_submit_node` will be preceded by a call to `apply_setup`
    /// and will eventually be followed by a call to `revert_setup`.
    fn render_submit_node(
        &self,
        _write_context: &mut RenderJobCommandBufferContext,
        _view_frame_index: ViewFrameIndex,
        _render_phase_index: RenderPhaseIndex,
        _submit_node_id: SubmitNodeId,
    ) -> RafxResult<()>;

    /// Called by `PreparedRenderData` in `write_view_phase` whenever the current `RenderFeatureIndex`
    /// changes **away** from this `RenderFeature`. This can be used to teardown pipelines or other expensive
    /// state changes that remained constant for previous `render_submit_node` calls.
    fn revert_setup(
        &self,
        _write_context: &mut RenderJobCommandBufferContext,
        _view_frame_index: ViewFrameIndex,
        _render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants;

    fn feature_index(&self) -> RenderFeatureIndex;
}
