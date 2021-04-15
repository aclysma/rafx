use crate::demo_feature::prepare::DemoPrepareJob;
use crate::demo_feature::{
    DemoRenderFeature, DemoRenderNode, DemoRenderNodeSet, ExtractedPerFrameNodeDemoData,
    ExtractedPerViewNodeDemoData,
};
use crate::DemoComponent;
use crate::TransformComponent;
use legion::*;
use rafx::nodes::{
    ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex,
    RenderJobExtractContext, RenderView,
};
use rafx_base::slab::RawSlabKey;

#[derive(Default)]
pub struct DemoExtractJob {}

impl ExtractJob for DemoExtractJob {
    //
    // This function is given the framepacket. This allows iterating across all visible objects.
    // Frame nodes will exist once per visible object, regardless of how many views it is visible in
    // View nodes will exist per visible object, per view that it's in. For every frame node, there
    // will be one or more view nodes. For every view node, there will be exactly one corresponding
    // frame node.
    //
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        log::debug!("extract_begin {}", self.feature_debug_name());

        let mut demo_render_nodes = extract_context
            .extract_resources
            .fetch_mut::<DemoRenderNodeSet>();
        let world = extract_context.extract_resources.fetch::<World>();

        //
        // Update the mesh render nodes. This could be done earlier as part of a system. (Could be
        // pulled from an ECS as in this example). The intent is that the extract process can use
        // visibility info to index directly into the render nodes.
        //
        let mut query = <(Read<TransformComponent>, Read<DemoComponent>)>::query();

        for (position_component, demo_component) in query.iter(&*world) {
            let render_node = demo_render_nodes
                .get_mut(&demo_component.render_node)
                .unwrap();

            // Set values here
            render_node.position = position_component.position;
            render_node.alpha = demo_component.alpha
        }

        //
        // Collect per-frame-node data from the render nodes. This could share collected data for
        // the same rendered object that will be rendered in multiple views
        //
        let per_frame_data: Vec<ExtractedPerFrameNodeDemoData> = frame_packet
            .frame_nodes(self.feature_index())
            .iter()
            .enumerate()
            .map(|(frame_node_index, frame_node)| {
                log::debug!(
                    "extract_frame_node {} {}",
                    self.feature_debug_name(),
                    frame_node_index
                );

                let render_node_index = frame_node.render_node_index();
                let render_node = RawSlabKey::<DemoRenderNode>::new(render_node_index);

                let demo_render_node = demo_render_nodes.demos.get_raw(render_node).unwrap();

                ExtractedPerFrameNodeDemoData {
                    position: demo_render_node.position,
                    alpha: demo_render_node.alpha,
                }
            })
            .collect();

        //
        // Collect per-view-node data. This would be any data we want to fetch that's unique to the
        // view we are drawing in. (In many cases there is no per-view data to extract and this
        // wouldn't be needed!). We'll do it here just to demonstrate.
        //
        let per_view_data: Vec<Vec<ExtractedPerViewNodeDemoData>> = views
            .iter()
            .map(|view| {
                let view_nodes = frame_packet.view_nodes(view, self.feature_index());
                if let Some(view_nodes) = view_nodes {
                    view_nodes
                        .iter()
                        .enumerate()
                        .map(|(view_node_index, view_node)| {
                            log::debug!(
                                "extract_view_nodes {} {} {:?}",
                                self.feature_debug_name(),
                                view_node_index,
                                per_frame_data[view_node.frame_node_index() as usize]
                            );

                            let per_frame_data =
                                &per_frame_data[view_node.frame_node_index() as usize];
                            ExtractedPerViewNodeDemoData {
                                alpha: per_frame_data.alpha,
                                position: per_frame_data.position,
                            }
                        })
                        .collect()
                } else {
                    Vec::default()
                }
            })
            .collect();

        //
        // Return a prepare job - this can be used to do any processing/binding async from the next
        // frame's simulation update
        //
        Box::new(DemoPrepareJob {
            per_frame_data,
            per_view_data,
        })
    }

    fn feature_debug_name(&self) -> &'static str {
        DemoRenderFeature::feature_debug_name()
    }
    fn feature_index(&self) -> RenderFeatureIndex {
        DemoRenderFeature::feature_index()
    }
}
