mod extract;
pub use extract::*;

mod prepare;
pub use prepare::*;
use crate::{RenderPhase, RenderView, RenderRegistry, RenderFeatureIndex, RenderPhaseIndex};

type SubmitNodeId = u32;
type SubmitNodeSortKey = u32;

#[derive(Copy, Clone)]
pub struct SubmitNode {
    feature_index: RenderFeatureIndex,
    submit_node_id: SubmitNodeId,
    sort_key: SubmitNodeSortKey,
}

// prepare produces a prepare result per feature per view per phase?
// merge the prepare results into per view per phase
// view/phase sorts nodes and prduces a command buffer
// PreparedViewPhaseData allows getting the command buffer

pub struct ViewSubmitNodes {
    // Index render phase index
    submit_nodes: Vec<Vec<SubmitNode>>,
    feature_index: RenderFeatureIndex
}

impl ViewSubmitNodes {
    pub fn new(feature_index: RenderFeatureIndex) -> Self {
        let submit_nodes = (0..RenderRegistry::registered_render_phase_count()).map(
            |_render_phase_index| Vec::new()
        ).collect();

        ViewSubmitNodes {
            submit_nodes,
            feature_index
        }
    }

    pub fn add_submit_node<RenderPhaseT: RenderPhase>(&mut self, submit_node_id: SubmitNodeId, sort_key: SubmitNodeSortKey) {
        log::debug!("add submit node render phase: {} feature: {} submit node id: {} sort key: {}", RenderPhaseT::render_phase_index(), self.feature_index, submit_node_id, sort_key);
        self.submit_nodes[RenderPhaseT::render_phase_index() as usize].push(SubmitNode {
            feature_index: self.feature_index,
            submit_node_id,
            sort_key
        });
    }

    pub fn submit_nodes(&self, render_phase_index: RenderPhaseIndex) -> &[SubmitNode] {
        &self.submit_nodes[render_phase_index as usize]
    }
}

pub struct FeatureSubmitNodes {
    // Index by view index
    submit_nodes: Vec<ViewSubmitNodes>
}

impl FeatureSubmitNodes {
    pub fn new(view_count: usize, feature_index: RenderFeatureIndex) -> Self {
        let submit_nodes = (0..view_count).map(
            |_view_index| ViewSubmitNodes::new(feature_index)
        ).collect();

        FeatureSubmitNodes {
            submit_nodes
        }
    }

    pub fn add_submit_node<RenderPhaseT: RenderPhase>(&mut self, view: &RenderView, submit_node_id: SubmitNodeId, sort_key: SubmitNodeSortKey) {
        self.submit_nodes[view.view_index()].add_submit_node::<RenderPhaseT>(submit_node_id, sort_key);
    }

    pub fn submit_nodes(&self, view: &RenderView, render_phase_index: RenderPhaseIndex) -> &[SubmitNode] {
        &self.submit_nodes[view.view_index()].submit_nodes(render_phase_index)
    }

    pub fn view_submit_nodes_mut(&mut self, view: &RenderView) -> &mut ViewSubmitNodes {
        &mut self.submit_nodes[view.view_index()]
    }
}

// pub struct MergedViewSubmitNodes {
//     // Sort by view index, then render phase index
//     submit_nodes: Vec<Vec<SubmitNode>>
// }
//
// impl MergedViewSubmitNodes {
//     pub fn new() -> Self {
//         MergedViewSubmitNodes {
//             submit_nodes: Default::default()
//         }
//     }
// }

// prepare job (which represents a feature) has to produce a command writer N times, one per view/render phase combo
// - the prepare job can wrap data in arcs and pass to a trait object that can handle callbacks
// - DestT is whatever type the end user wants to store draw calls
// OR - prepare job just returns merged submit nodes and we provide some other solution for
//      turning a completed prepare job into a "feature renderer" that gets data piped to it

// trait FeatureSubmitNodeVisitor<DestT> {
//     fn apply_setup(&self, dest: &mut DestT);
//     fn render_element(&self, dest: &mut DestT, index: u32);
//     fn revert_setup(&self, dest: &mut DestT);
// }
//
// struct CommandBufferWriterSet<DestT> {
//     // Index by feature
//     writers: Vec<&'a CommandBufferWriter>
// }

pub struct MergedFrameSubmitNodes {
    // Sort by view index, then render phase index
    merged_submit_nodes: Vec<Vec<Vec<SubmitNode>>>
}

impl MergedFrameSubmitNodes {
    pub fn new(
        feature_submit_nodes: Vec<FeatureSubmitNodes>,
        views: &[&RenderView]
    ) -> MergedFrameSubmitNodes {

        let mut merged_submit_nodes = Vec::with_capacity(views.len());

        //TODO: Can probably merge/sort views/render phase pairs in parallel if needed
        for view in views {
            let mut combined_nodes_for_view = Vec::with_capacity(RenderRegistry::registered_render_phase_count() as usize);

            for render_phase_index in 0..RenderRegistry::registered_render_phase_count() {
                let mut combined_nodes_for_phase = Vec::new();

                println!("Merging submit nodes for view: {} phase: {}", view.view_index(), render_phase_index);
                // callback to feature to do sort?
                for fsn in &feature_submit_nodes {
                    let submit_nodes = fsn.submit_nodes(view, render_phase_index);

                    //TODO: Sort as we push into the vec?
                    for submit_node in submit_nodes {
                        println!("submit node feature: {} id: {}", submit_node.feature_index, submit_node.submit_node_id);
                        combined_nodes_for_phase.push(*submit_node);
                    }
                }

                //TODO: Sort the render nodes (probably configurable at the render phase level)

                combined_nodes_for_view.push(combined_nodes_for_phase)

                // //TODO: Produce command buffers?
                // let mut previous_node_feature_index : i32 = -1;
                // for submit_node in combined_submit_nodes {
                //     if submit_node.feature_index as i32 != previous_node_feature_index {
                //         if previous_node_feature_index != -1 {
                //             // call revert setup
                //             log::debug!("revert setup for feature {}", previous_node_feature_index);
                //         }
                //
                //         // call apply setup
                //         log::debug!("apply setup for feature {}", submit_node.feature_index);
                //     }
                //
                //     log::debug!("draw render node feature: {} node id: {}", submit_node.feature_index, submit_node.submit_node_id);
                //     previous_node_feature_index = submit_node.feature_index as i32;
                // }
                //
                // if previous_node_feature_index != -1 {
                //     // call revert setup
                //     log::debug!("revert setup for feature: {}", previous_node_feature_index);
                // }
            }

            merged_submit_nodes.push(combined_nodes_for_view);
        }

        MergedFrameSubmitNodes {
            merged_submit_nodes
        }
    }
}


// Need a way to pass along prepared data into a submit handler. The submit handler will be called
// based on submit nodes. This will produce command buffers for each view/render stage combination
