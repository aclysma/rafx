use crate::render_features::{
    RenderFeatureDebugConstants, RenderFeatureIndex, RenderJobBeginExecuteGraphContext,
    RenderJobCommandBufferContext, RenderPhaseIndex, SubmitNodeId, ViewFrameIndex,
};
use rafx_api::RafxResult;

pub struct BeginSubmitNodeBatchArgs {
    pub view_frame_index: ViewFrameIndex,
    pub render_phase_index: RenderPhaseIndex,
    pub feature_changed: bool,
    pub sort_key: u32,
}

pub struct RenderSubmitNodeArgs {
    pub view_frame_index: ViewFrameIndex,
    pub render_phase_index: RenderPhaseIndex,
    pub submit_node_id: SubmitNodeId,
}

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

    /// Called by `PreparedRenderData` in `write_view_phase` whenever the current `RenderFeatureIndex`
    /// changes **into** this `RenderFeature` OR when the `sort_key` changes. This can be used to
    /// setup pipelines or other expensive state changes that can remain constant for subsequent
    /// `render_submit_node` calls.
    fn begin_submit_node_batch(
        &self,
        _write_context: &mut RenderJobCommandBufferContext,
        _args: BeginSubmitNodeBatchArgs,
    ) -> RafxResult<()> {
        Ok(())
    }

    /// Called by `PreparedRenderData` in `write_view_phase` for each `RenderFeatureSubmitNode` associated
    /// with this `RenderFeature`. This is normally where the actual GPU draw commands are implemented for
    /// the `RenderFeature`. Each series of calls to `render_submit_node` will be preceded by a single call to
    /// `begin_submit_node_batch`.
    fn render_submit_node(
        &self,
        _write_context: &mut RenderJobCommandBufferContext,
        args: RenderSubmitNodeArgs,
    ) -> RafxResult<()>;

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants;

    fn feature_index(&self) -> RenderFeatureIndex;
}
