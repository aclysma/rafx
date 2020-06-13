use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use renderer_base::{RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex, FramePacket, DefaultPrepareJobImpl, PerFrameNode, PerViewNode, RenderFeature};
use crate::DemoCommandWriterContext;
use crate::features::demo::{DemoRenderFeature, ExtractedDemoData};
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use glam::Vec3;
use crate::features::demo::write::DemoCommandWriter;

pub struct DemoPrepareJobImpl {
    pub(super) per_frame_data: Vec<ExtractedDemoData>,
    pub(super) per_view_data: Vec<Vec<ExtractedDemoData>>,
}

impl DefaultPrepareJobImpl<DemoCommandWriterContext> for DemoPrepareJobImpl {
    fn prepare_begin(
        &mut self,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {
        log::debug!("prepare_begin {}", self.feature_debug_name());
    }

    fn prepare_frame_node(
        &mut self,
        _frame_node: PerFrameNode,
        frame_node_index: u32,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {
        log::debug!(
            "prepare_frame_node {} {}",
            self.feature_debug_name(),
            frame_node_index
        );
    }

    fn prepare_view_node(
        &mut self,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    ) {
        log::debug!(
            "prepare_view_node {} {} {:?}",
            self.feature_debug_name(),
            view_node_index,
            self.per_frame_data[view_node.frame_node_index() as usize]
        );

        // This can read per-frame and per-view data
        //let extracted_data = &self.per_frame_data[view_node.frame_node_index() as usize];
        let extracted_data =
            &self.per_view_data[view.view_index() as usize][view_node_index as usize];

        if extracted_data.alpha >= 1.0 {
            submit_nodes.add_submit_node::<DrawOpaqueRenderPhase>(view_node_index, 0, 0.0);
        } else {
            let distance_from_camera = Vec3::length(extracted_data.position - view.eye_position());
            submit_nodes.add_submit_node::<DrawTransparentRenderPhase>(
                view_node_index,
                0,
                distance_from_camera,
            );
        }
    }

    fn prepare_view_finalize(
        &mut self,
        _view: &RenderView,
        _submit_nodes: &mut ViewSubmitNodes,
    ) {
        log::debug!("prepare_view_finalize {}", self.feature_debug_name());
    }

    fn prepare_frame_finalize(
        self,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<DemoCommandWriterContext>> {
        log::debug!("prepare_frame_finalize {}", self.feature_debug_name());
        Box::new(DemoCommandWriter {})
    }

    fn feature_debug_name(&self) -> &'static str {
        DemoRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        DemoRenderFeature::feature_index()
    }
}
