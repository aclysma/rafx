mod extract;
pub use extract::*;

mod prepare;
pub use prepare::*;
use crate::{RenderPhase, RenderView, RenderRegistry, RenderFeatureIndex, RenderPhaseIndex};

type SubmitNodeId = u32;
type SubmitNodeSortKey = u32;

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

    pub fn view_submit_nodes(&self, view: &RenderView) -> &ViewSubmitNodes {
        &self.submit_nodes[view.view_index()]
    }

    pub fn view_submit_nodes_mut(&mut self, view: &RenderView) -> &mut ViewSubmitNodes {
        &mut self.submit_nodes[view.view_index()]
    }
}

pub struct MergedViewSubmitNodes {
    // Sort by view index, then render phase index
    submit_nodes: Vec<Vec<SubmitNode>>
}

impl MergedViewSubmitNodes {
    pub fn new() -> Self {
        MergedViewSubmitNodes {
            submit_nodes: Default::default()
        }
    }
}

pub struct MergedFrameSubmitNodes {
    // Sort by view index, then render phase index
    merged_view_submit_nodes: Vec<Vec<SubmitNode>>
}

// impl MergedFrameSubmitNodes {
//     pub fn new() -> Self {
//         MergedFrameSubmitNodes {
//             merged_view_submit_nodes: Default::default()
//         }
//     }
// }

impl MergedFrameSubmitNodes {
    pub fn new(feature_submit_nodes: Vec<FeatureSubmitNodes>, views: &[&RenderView]) -> MergedFrameSubmitNodes {
        //TODO: Can probably run views in parallel
        for view in views {
            for render_phase_index in 0..RenderRegistry::registered_render_phase_count() {
                let mut combined_submit_nodes = Vec::new();

                println!("Merging submit nodes for view: {} phase: {}", view.view_index(), render_phase_index);
                // callback to feature to do sort?
                for fsn in &feature_submit_nodes {
                    let view_submit_nodes = fsn.view_submit_nodes(view);
                    let submit_nodes = view_submit_nodes.submit_nodes(render_phase_index);

                    //TODO: Sort as we push into the vec?
                    for submit_node in submit_nodes {
                        println!("submit node feature: {} id: {}", submit_node.feature_index, submit_node.submit_node_id);
                        combined_submit_nodes.push(submit_node);
                    }
                }

                //TODO: Sort the render nodes

                //TODO: Produce command buffers?
            }
        }

        MergedFrameSubmitNodes {
            merged_view_submit_nodes: Default::default()
        }
    }
}


// Need a way to pass along prepared data into a submit handler. The submit handler will be called
// based on submit nodes. This will produce command buffers for each view/render stage combination