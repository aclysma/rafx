use crate::{
    RenderPhase, RenderView, RenderRegistry, RenderFeatureIndex, RenderPhaseIndex, RenderViewIndex,
};
use fnv::FnvHashMap;

pub type SubmitNodeId = u32;
pub type SubmitNodeSortKey = u32;

#[derive(Copy, Clone)]
pub struct SubmitNode {
    feature_index: RenderFeatureIndex,
    submit_node_id: SubmitNodeId,
    sort_key: SubmitNodeSortKey,
    distance_from_camera: f32,
}

impl SubmitNode {
    pub fn feature_index(&self) -> RenderFeatureIndex {
        self.feature_index
    }

    pub fn submit_node_id(&self) -> SubmitNodeId {
        self.submit_node_id
    }

    pub fn sort_key(&self) -> SubmitNodeSortKey {
        self.sort_key
    }
}

pub struct ViewSubmitNodes {
    // Index render phase index
    submit_nodes: Vec<Vec<SubmitNode>>,
    feature_index: RenderFeatureIndex,
}

impl ViewSubmitNodes {
    pub fn new(feature_index: RenderFeatureIndex) -> Self {
        let submit_nodes = (0..RenderRegistry::registered_render_phase_count())
            .map(|_render_phase_index| Vec::new())
            .collect();

        ViewSubmitNodes {
            submit_nodes,
            feature_index,
        }
    }

    pub fn add_submit_node<RenderPhaseT: RenderPhase>(
        &mut self,
        submit_node_id: SubmitNodeId,
        sort_key: SubmitNodeSortKey,
        distance_from_camera: f32,
    ) {
        log::debug!("add submit node render phase: {} feature: {} submit node id: {} sort key: {} distance: {}", RenderPhaseT::render_phase_index(), self.feature_index, submit_node_id, sort_key, distance_from_camera);
        self.submit_nodes[RenderPhaseT::render_phase_index() as usize].push(SubmitNode {
            feature_index: self.feature_index,
            submit_node_id,
            sort_key,
            distance_from_camera,
        });
    }

    pub fn submit_nodes(
        &self,
        render_phase_index: RenderPhaseIndex,
    ) -> &[SubmitNode] {
        &self.submit_nodes[render_phase_index as usize]
    }
}

pub struct FeatureSubmitNodes {
    // Index by view index
    submit_nodes: Vec<ViewSubmitNodes>,
}

impl FeatureSubmitNodes {
    pub fn new(
        view_count: usize,
        feature_index: RenderFeatureIndex,
    ) -> Self {
        let submit_nodes = (0..view_count)
            .map(|_view_index| ViewSubmitNodes::new(feature_index))
            .collect();

        FeatureSubmitNodes { submit_nodes }
    }

    pub fn add_submit_node<RenderPhaseT: RenderPhase>(
        &mut self,
        view: &RenderView,
        submit_node_id: SubmitNodeId,
        sort_key: SubmitNodeSortKey,
        distance_from_camera: f32,
    ) {
        self.submit_nodes[view.view_index() as usize].add_submit_node::<RenderPhaseT>(
            submit_node_id,
            sort_key,
            distance_from_camera,
        );
    }

    pub fn submit_nodes(
        &self,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> &[SubmitNode] {
        &self.submit_nodes[view.view_index() as usize].submit_nodes(render_phase_index)
    }

    pub fn view_submit_nodes_mut(
        &mut self,
        view: &RenderView,
    ) -> &mut ViewSubmitNodes {
        &mut self.submit_nodes[view.view_index() as usize]
    }
}

pub struct MergedFrameSubmitNodes {
    // Sort by view index, then render phase index
    merged_submit_nodes: FnvHashMap<RenderViewIndex, Vec<Vec<SubmitNode>>>,
}

impl MergedFrameSubmitNodes {
    pub fn new(
        feature_submit_nodes: Vec<FeatureSubmitNodes>,
        views: &[&RenderView],
        registry: &RenderRegistry,
    ) -> MergedFrameSubmitNodes {
        let mut merged_submit_nodes = FnvHashMap::default();

        //TODO: Can probably merge/sort views/render phase pairs in parallel if needed
        for view in views {
            let mut combined_nodes_for_view =
                Vec::with_capacity(RenderRegistry::registered_render_phase_count() as usize);

            for render_phase_index in 0..RenderRegistry::registered_render_phase_count() {
                let mut combined_nodes_for_phase = Vec::new();

                log::debug!(
                    "Merging submit nodes for view: {} phase: {}",
                    view.view_index(),
                    render_phase_index
                );
                for fsn in &feature_submit_nodes {
                    let submit_nodes = fsn.submit_nodes(view, render_phase_index);

                    //TODO: Sort as we push into the vec?
                    for submit_node in submit_nodes {
                        log::debug!(
                            "submit node feature: {} id: {}",
                            submit_node.feature_index,
                            submit_node.submit_node_id
                        );
                        combined_nodes_for_phase.push(*submit_node);
                    }
                }

                let combined_nodes_for_phase =
                    registry.sort_submit_nodes(render_phase_index, combined_nodes_for_phase);

                //TODO: Sort the render nodes (probably configurable at the render phase level)
                // callback to feature to do sort?
                combined_nodes_for_view.push(combined_nodes_for_phase)
            }

            merged_submit_nodes.insert(view.view_index(), combined_nodes_for_view);
        }

        MergedFrameSubmitNodes {
            merged_submit_nodes,
        }
    }

    pub fn submit_nodes<PhaseT: RenderPhase>(
        &self,
        view: &RenderView,
    ) -> &[SubmitNode] {
        &self.merged_submit_nodes[&view.view_index()][PhaseT::render_phase_index() as usize]
    }
}
