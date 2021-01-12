use crate::demo_feature::{
    DemoRenderFeature, ExtractedPerFrameNodeDemoData, ExtractedPerViewNodeDemoData,
    PreparedPerSubmitNodeDemoData,
};
use crate::DemoWriteContext;
use rafx_api::RafxResult;
use rafx_nodes::{
    FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderPhaseIndex, RenderView,
    SubmitNodeId,
};

pub struct DemoCommandWriter {
    pub(super) per_frame_data: Vec<ExtractedPerFrameNodeDemoData>,
    pub(super) per_view_data: Vec<Vec<ExtractedPerViewNodeDemoData>>,
    pub(super) per_submit_node_data: Vec<PreparedPerSubmitNodeDemoData>,
}

impl FeatureCommandWriter<DemoWriteContext> for DemoCommandWriter {
    fn apply_setup(
        &self,
        _write_context: &mut DemoWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        log::debug!(
            "apply_setup {} view: {}",
            self.feature_debug_name(),
            view.debug_name()
        );

        Ok(())
    }

    fn render_element(
        &self,
        _write_context: &mut DemoWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> RafxResult<()> {
        log::info!(
            "render_element {} view: {} id: {}",
            self.feature_debug_name(),
            view.debug_name(),
            index
        );

        // This demonstrates accessing data that was extracted or prepared
        let submit_node_data = &self.per_submit_node_data[index as usize];
        let _frame_node_data = &self.per_frame_data[submit_node_data.frame_node_index as usize];
        let _view_node_data = &self.per_view_data[submit_node_data.view_node_index as usize];

        Ok(())
    }

    fn revert_setup(
        &self,
        _write_context: &mut DemoWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        log::debug!(
            "revert_setup {} view: {}",
            self.feature_debug_name(),
            view.debug_name()
        );
        Ok(())
    }

    fn feature_debug_name(&self) -> &'static str {
        DemoRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        DemoRenderFeature::feature_index()
    }
}
