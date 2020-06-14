use crate::features::demo::{ExtractedDemoData, DemoRenderNodeSet, DemoRenderFeature, DemoRenderNode};
use crate::{DemoExtractContext, DemoWriteContext, PositionComponent, DemoComponent, DemoPrepareContext};
use renderer_base::{DefaultExtractJobImpl, FramePacket, RenderView, PerViewNode, PrepareJob, DefaultPrepareJob, RenderFeatureIndex, RenderFeature, PerFrameNode};
use renderer_base::slab::RawSlabKey;
use crate::features::demo::prepare::DemoPrepareJobImpl;

#[derive(Default)]
pub struct DemoExtractJobImpl {
    per_frame_data: Vec<ExtractedDemoData>,
    per_view_data: Vec<Vec<ExtractedDemoData>>,
}

impl DefaultExtractJobImpl<DemoExtractContext, DemoPrepareContext, DemoWriteContext> for DemoExtractJobImpl {
    fn extract_begin(
        &mut self,
        _extract_context: &DemoExtractContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) {
        log::debug!("extract_begin {}", self.feature_debug_name());
        self.per_frame_data
            .reserve(frame_packet.frame_node_count(self.feature_index()) as usize);

        self.per_view_data.reserve(views.len());
        for view in views {
            self.per_view_data.push(Vec::with_capacity(
                frame_packet.view_node_count(view, self.feature_index()) as usize,
            ));
        }
    }

    fn extract_frame_node(
        &mut self,
        source: &DemoExtractContext,
        frame_node: PerFrameNode,
        frame_node_index: u32,
    ) {
        log::debug!(
            "extract_frame_node {} {}",
            self.feature_debug_name(),
            frame_node_index
        );

        let render_node_index = frame_node.render_node_index();
        let render_node_handle = RawSlabKey::<DemoRenderNode>::new(render_node_index);

        let demo_nodes = source.resources.get::<DemoRenderNodeSet>().unwrap();
        let demo_render_node = demo_nodes.demos.get(render_node_handle).unwrap();
        let position_component = source
            .world
            .get_component::<PositionComponent>(demo_render_node.entity)
            .unwrap();
        let demo_component = source
            .world
            .get_component::<DemoComponent>(demo_render_node.entity)
            .unwrap();

        self.per_frame_data.push(ExtractedDemoData {
            position: position_component.position,
            alpha: demo_component.alpha,
        });
    }

    fn extract_view_node(
        &mut self,
        _extract_context: &DemoExtractContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
    ) {
        log::debug!(
            "extract_view_nodes {} {} {:?}",
            self.feature_debug_name(),
            view_node_index,
            self.per_frame_data[view_node.frame_node_index() as usize]
        );
        let frame_data = self.per_frame_data[view_node.frame_node_index() as usize].clone();
        self.per_view_data[view.view_index() as usize].push(frame_data);
    }

    fn extract_view_finalize(
        &mut self,
        _extract_context: &DemoExtractContext,
        _view: &RenderView,
    ) {
        log::debug!("extract_view_finalize {}", self.feature_debug_name());
    }

    fn extract_frame_finalize(
        self,
        _extract_context: &DemoExtractContext,
    ) -> Box<dyn PrepareJob<DemoPrepareContext, DemoWriteContext>> {
        log::debug!("extract_frame_finalize {}", self.feature_debug_name());

        let prepare_impl = DemoPrepareJobImpl {
            per_frame_data: self.per_frame_data,
            per_view_data: self.per_view_data,
        };

        Box::new(DefaultPrepareJob::new(prepare_impl))
    }

    fn feature_debug_name(&self) -> &'static str {
        DemoRenderFeature::feature_debug_name()
    }
    fn feature_index(&self) -> RenderFeatureIndex {
        DemoRenderFeature::feature_index()
    }
}
