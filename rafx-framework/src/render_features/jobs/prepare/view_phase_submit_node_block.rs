use crate::render_features::render_features_prelude::*;

/// The `ID` of a `SubmitNode` in a specific `RenderFeature` in the current frame.
pub type SubmitNodeId = u32;

/// A generic key usable by the `SubmitNodeSortFunction` for the purpose of sorting the collection of
/// `RenderFeatureSubmitNode` in a particular `ViewPhase`. This can be used to minimize state changes
/// in the rendering pipeline, e.g. by setting the bits of the key so that higher bits represent more
/// expensive state changes like shaders and lower bits represent cheaper state changes like uniforms.
///
/// Example: https://web.archive.org/web/20210110113523/https://realtimecollisiondetection.net/blog/?p=86
pub type SubmitNodeSortKey = u32;

/// The sort function used by a particular `RenderPhase`. This is usually one of the following:
/// 1. front-to-back
/// 2. back-to-front
/// 3. by feature index
/// 4. unsorted
pub type SubmitNodeSortFunction = fn(&mut Vec<RenderFeatureSubmitNode>);

/// A combination of a particular `RenderView` and `RenderPhase`. The `ViewPhaseSubmitNodeBlock`s
/// in the `PreparedRenderData` are indexed by the `ViewPhase`.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct ViewPhase {
    pub view_index: RenderViewIndex,
    pub phase_index: RenderPhaseIndex,
}

/// The `ViewPhaseSubmitNodeBlock` is a collection of `RenderFeatureSubmitNode` associated with
/// a particular `RenderView`, and `RenderPhase`. In other words, the `ViewPhaseSubmitNodeBlock`
/// is the combination of all the feature-specific `RenderFeatureSubmitNodeBlock`s with the same
/// `RenderView` and `RenderPhase`.
#[derive(Debug)]
pub struct ViewPhaseSubmitNodeBlock {
    view_phase: ViewPhase,
    submit_nodes: Vec<RenderFeatureSubmitNode>,
}

impl ViewPhaseSubmitNodeBlock {
    pub fn new(
        view_phase: ViewPhase,
        num_submit_nodes: usize,
    ) -> Self {
        Self {
            view_phase,
            submit_nodes: Vec::with_capacity(num_submit_nodes),
        }
    }

    pub fn view_phase(&self) -> &ViewPhase {
        &self.view_phase
    }

    pub fn len(&self) -> usize {
        self.submit_nodes.len()
    }

    pub fn push_submit_node(
        &mut self,
        submit_node: RenderFeatureSubmitNode,
    ) {
        self.submit_nodes.push(submit_node)
    }

    pub fn sort_submit_nodes(
        &mut self,
        sort_function: SubmitNodeSortFunction,
    ) {
        sort_function(&mut self.submit_nodes)
    }

    pub fn submit_nodes(&self) -> &[RenderFeatureSubmitNode] {
        self.submit_nodes.as_slice()
    }
}

/// A type-erased struct representing some `RenderFeature`'s `SubmitNode`. The `PreparedRenderData`
/// will iterate through the sorted slice of `RenderFeatureSubmitNode` in a `ViewPhaseSubmitNodeBlock`
/// and call the functions on the `RenderFeatureWriteJob` specified by the `RenderFeatureIndex`.
#[derive(Copy, Clone, Debug)]
pub struct RenderFeatureSubmitNode {
    feature_index: RenderFeatureIndex,
    submit_node_id: SubmitNodeId,
    sort_key: SubmitNodeSortKey,
    distance: f32,
}

impl RenderFeatureSubmitNode {
    pub fn new(
        feature_index: RenderFeatureIndex,
        submit_node_id: SubmitNodeId,
        sort_key: SubmitNodeSortKey,
        distance: f32,
    ) -> Self {
        Self {
            feature_index,
            submit_node_id,
            sort_key,
            distance,
        }
    }

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
