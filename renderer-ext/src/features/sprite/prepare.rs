use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use renderer_base::{RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex, FramePacket, DefaultPrepareJobImpl, PerFrameNode, PerViewNode, RenderFeature};
use crate::features::sprite::{SpriteRenderFeature, ExtractedSpriteData};
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use glam::Vec3;
use super::SpriteCommandWriter;
use crate::CommandWriterContext;

pub struct SpritePrepareJobImpl {
    pub(super) per_frame_data: Vec<ExtractedSpriteData>,
}

impl DefaultPrepareJobImpl<CommandWriterContext> for SpritePrepareJobImpl {
    fn prepare_begin(
        &mut self,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {

    }

    fn prepare_frame_node(
        &mut self,
        _frame_node: PerFrameNode,
        frame_node_index: u32,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {

    }

    fn prepare_view_node(
        &mut self,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    ) {
        // This can read per-frame and per-view data
        let extracted_data =
            &self.per_frame_data[view_node.frame_node_index() as usize];

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

    }

    fn prepare_frame_finalize(
        self,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<CommandWriterContext>> {
        Box::new(SpriteCommandWriter {})
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}
