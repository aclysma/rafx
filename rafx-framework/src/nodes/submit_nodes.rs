use super::{
    RenderFeatureIndex, RenderPhase, RenderPhaseIndex, RenderPhaseMask, RenderRegistry, RenderView,
    RenderViewIndex,
};
use fnv::FnvHashMap;

pub type SubmitNodeId = u32;
pub type SubmitNodeSortKey = u32;

#[derive(Copy, Clone, Debug)]
pub struct SubmitNode {
    feature_index: RenderFeatureIndex,
    submit_node_id: SubmitNodeId,
    sort_key: SubmitNodeSortKey,
    distance: f32,
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

    pub fn distance(&self) -> f32 {
        self.distance
    }
}

#[derive(Debug)]
pub struct ViewSubmitNodes {
    // Index render phase index
    submit_nodes: Vec<Vec<SubmitNode>>,
    feature_index: RenderFeatureIndex,
    render_phase_mask: RenderPhaseMask,
}

impl ViewSubmitNodes {
    pub fn new(
        feature_index: RenderFeatureIndex,
        render_phase_mask: RenderPhaseMask,
    ) -> Self {
        let submit_nodes = (0..RenderRegistry::registered_render_phase_count())
            .map(|_render_phase_index| Vec::new())
            .collect();

        ViewSubmitNodes {
            submit_nodes,
            feature_index,
            render_phase_mask,
        }
    }

    pub fn add_submit_node<RenderPhaseT: RenderPhase>(
        &mut self,
        submit_node_id: SubmitNodeId,
        sort_key: SubmitNodeSortKey,
        distance: f32,
    ) {
        if self.render_phase_mask.is_included::<RenderPhaseT>() {
            log::trace!("add submit node render phase: {} feature: {} submit node id: {} sort key: {} distance: {}", RenderPhaseT::render_phase_index(), self.feature_index, submit_node_id, sort_key, distance);
            self.submit_nodes[RenderPhaseT::render_phase_index() as usize].push(SubmitNode {
                feature_index: self.feature_index,
                submit_node_id,
                sort_key,
                distance,
            });
        }
    }

    pub fn submit_nodes(
        &self,
        render_phase_index: RenderPhaseIndex,
    ) -> &[SubmitNode] {
        &self.submit_nodes[render_phase_index as usize]
    }
}

#[derive(Default, Debug)]
pub struct FeatureSubmitNodes {
    submit_nodes: FnvHashMap<RenderViewIndex, ViewSubmitNodes>,
}

impl FeatureSubmitNodes {
    pub fn add_submit_nodes_for_view(
        &mut self,
        view: &RenderView,
        submit_nodes: ViewSubmitNodes,
    ) {
        self.submit_nodes.insert(view.view_index(), submit_nodes);
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct ViewPhase {
    pub view_index: RenderViewIndex,
    pub phase_index: RenderPhaseIndex,
}

pub struct MergedFrameSubmitNodes {
    // Sort by view index, then render phase index
    merged_submit_nodes: FnvHashMap<ViewPhase, Vec<SubmitNode>>,
}

impl MergedFrameSubmitNodes {
    pub fn new(
        feature_submit_nodes: Vec<FeatureSubmitNodes>, // index by feature, then view
        registry: &RenderRegistry,
    ) -> MergedFrameSubmitNodes {
        // Group all the render nodes by view/phase
        let mut submit_nodes_by_view_phase = FnvHashMap::default();
        for nodes in feature_submit_nodes {
            for (view_index, view_nodes) in nodes.submit_nodes {
                for (phase_index, phase_nodes) in view_nodes.submit_nodes.into_iter().enumerate() {
                    let view_phase = ViewPhase {
                        view_index,
                        phase_index: phase_index as RenderPhaseIndex,
                    };

                    submit_nodes_by_view_phase
                        .entry(view_phase)
                        .or_insert_with(Vec::<Vec<SubmitNode>>::new)
                        .push(phase_nodes);
                }
            }
        }

        // For each view/phase pair, merge the nodes that each feature produced and sort them
        let mut merged_submit_nodes = FnvHashMap::default();
        for (view_phase, view_nodes) in submit_nodes_by_view_phase {
            let mut all_nodes = vec![];
            for mut nodes in view_nodes {
                all_nodes.append(&mut nodes);
            }

            let all_nodes = registry.sort_submit_nodes(view_phase.phase_index, all_nodes);

            merged_submit_nodes.insert(view_phase, all_nodes);
        }

        MergedFrameSubmitNodes {
            merged_submit_nodes,
        }
    }

    pub fn submit_nodes<PhaseT: RenderPhase>(
        &self,
        view: &RenderView,
    ) -> &[SubmitNode] {
        let view_phase = ViewPhase {
            view_index: view.view_index(),
            phase_index: PhaseT::render_phase_index(),
        };

        let nodes = self.merged_submit_nodes.get(&view_phase);
        if let Some(nodes) = nodes {
            nodes
        } else {
            &[]
        }
    }
}
