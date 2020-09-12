use renderer_nodes::{RenderFeatureIndex, RenderFeature, SubmitNodeId, FeatureCommandWriter, RenderView, RenderPhaseIndex};
use crate::DemoWriteContext;
use crate::demo_feature::DemoRenderFeature;

pub struct DemoCommandWriter {}

impl FeatureCommandWriter<DemoWriteContext> for DemoCommandWriter {
    fn apply_setup(
        &self,
        _write_context: &mut DemoWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
        log::debug!(
            "apply_setup {} view: {}",
            self.feature_debug_name(),
            view.debug_name()
        );
    }

    fn render_element(
        &self,
        _write_context: &mut DemoWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) {
        log::info!(
            "render_element {} view: {} id: {}",
            self.feature_debug_name(),
            view.debug_name(),
            index
        );
    }

    fn revert_setup(
        &self,
        _write_context: &mut DemoWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
        log::debug!(
            "revert_setup {} view: {}",
            self.feature_debug_name(),
            view.debug_name()
        );
    }

    fn feature_debug_name(&self) -> &'static str {
        DemoRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        DemoRenderFeature::feature_index()
    }
}
