use crate::demo_feature::write::DemoCommandWriter;
use crate::demo_feature::{
    DemoRenderFeature, ExtractedPerFrameNodeDemoData, ExtractedPerViewNodeDemoData,
    PreparedPerSubmitNodeDemoData,
};
use crate::demo_phases::*;
use glam::Vec3;
use rafx::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PrepareJob, RenderFeature,
    RenderFeatureIndex, RenderJobPrepareContext, RenderView, RenderViewIndex, SubmitNodeId,
    ViewSubmitNodes,
};

pub struct DemoPrepareJob {
    pub(super) per_frame_data: Vec<ExtractedPerFrameNodeDemoData>,
    pub(super) per_view_data: Vec<Vec<ExtractedPerViewNodeDemoData>>,
}

impl PrepareJob for DemoPrepareJob {
    fn prepare(
        self: Box<Self>,
        _prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes) {
        //
        // The submit node struct will combine all submit nodes across all views for this feature.
        // It later gets merged with render nodes from other features and sorted. This is useful
        // for example to do depth-sorting of transparent objects, even if those objects are being
        // generated by different features
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        let mut per_submit_node_data = Vec::default();

        for (view_index, view) in views.iter().enumerate() {
            // The submit nodes for this view
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());

            let view_nodes = frame_packet.view_nodes(view, self.feature_index());
            if let Some(view_nodes) = view_nodes {
                for (view_node_index, view_node) in view_nodes.iter().enumerate() {
                    //
                    // Lookup the data we extracted. We can get per-frame or per-view data as needed.
                    // In this example we put the same data in both so one of them will go unused.
                    //
                    let _per_frame_extracted_data =
                        &self.per_frame_data[view_node.frame_node_index() as usize];
                    let per_view_extracted_data =
                        &self.per_view_data[view_index][view_node_index as usize];

                    let submit_node_index = per_submit_node_data.len();
                    per_submit_node_data.push(PreparedPerSubmitNodeDemoData {
                        position: per_view_extracted_data.position,
                        alpha: per_view_extracted_data.alpha,
                        frame_node_index: view_node.frame_node_index(),
                        view_index: view_index as RenderViewIndex,
                    });

                    //
                    // Add submit nodes for all items we want to render. We will check alpha and
                    // add a submit node for the appropriate phase (transparent vs. non-transparent)
                    //
                    // The parameters passed to add_submit node are:
                    // - submit_node_id: This is a per-feature value that can be used however you
                    //   like. In this case since we are drawing one thing per view node, we will
                    //   let it be the view node index.
                    // - The sort key and distance are user-defined and can be used by the
                    //   render phase in any way a user wants. Distance might be distance from camera
                    //   to do depth sorting, and sort key could be hashed in a way to allow
                    //   batching like-materials and descriptor sets
                    //
                    if per_view_extracted_data.alpha >= 1.0 {
                        // Add to the opaque render phase if no alpha-blending is needed. The sort
                        // key and distance from camera are user-driven and can be used in whatever
                        // way makes sense for the phase.
                        view_submit_nodes.add_submit_node::<DemoOpaqueRenderPhase>(
                            submit_node_index as SubmitNodeId,
                            0,
                            0.0,
                        );
                    } else {
                        // Add to the transparent phase (which can sort based on depth)
                        let distance =
                            Vec3::length(per_view_extracted_data.position - view.eye_position());
                        view_submit_nodes.add_submit_node::<DemoTransparentRenderPhase>(
                            submit_node_index as SubmitNodeId,
                            0,
                            distance,
                        );
                    }
                }
            }

            //
            // When we get all the nodes for the view, merge them into the feature submit nodes.
            // (This design allows doing per-view processing in parallel)
            //
            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        //
        // Create the writer that will be called to write a submit node to a command buffer. Extra
        // data can be passed along to it.
        //
        let writer = DemoCommandWriter {
            per_frame_data: self.per_frame_data,
            per_view_data: self.per_view_data,
            per_submit_node_data,
        };

        (Box::new(writer), submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        DemoRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        DemoRenderFeature::feature_index()
    }
}
