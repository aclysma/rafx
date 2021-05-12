use crate::render_features::render_features_prelude::*;

/// The `SubmitNodeBlock` is a collection of `SubmitNode` associated with a particular `RenderFeature`,
/// `RenderView`, and `RenderPhase`. There should be a 1:1 mapping between `SubmitNode`s and draw calls
/// from the `RenderFeature`'s `WriteJob`. The `Renderer` will combine all `SubmitNodeBlock`s sharing the
/// same `RenderView` and `RenderPhase` into a sorted `ViewPhaseSubmitNodeBlock`.
pub struct SubmitNodeBlock<SubmitPacketDataT: SubmitPacketData> {
    feature_index: RenderFeatureIndex,
    render_phase: RenderPhaseIndex,
    submit_nodes: AtomicOnceCellStack<SubmitNode<SubmitPacketDataT::SubmitNodeData>>,
}

impl<SubmitPacketDataT: 'static + Sync + Send + SubmitPacketData>
    SubmitNodeBlock<SubmitPacketDataT>
{
    pub fn len(&self) -> usize {
        self.submit_nodes.len()
    }

    pub fn with_capacity<RenderPhaseT: RenderPhase>(
        view: &RenderView,
        num_submit_nodes: usize,
    ) -> Self {
        Self {
            feature_index: SubmitPacketDataT::RenderFeature::feature_index(),
            render_phase: RenderPhaseT::render_phase_index(),
            submit_nodes: AtomicOnceCellStack::with_capacity(
                if view.phase_is_relevant::<RenderPhaseT>() {
                    num_submit_nodes
                } else {
                    0
                },
            ),
        }
    }

    pub fn push_submit_node(
        &self,
        data: SubmitPacketDataT::SubmitNodeData,
        sort_key: SubmitNodeSortKey,
        distance: f32,
    ) -> SubmitNodeId {
        self.submit_nodes.push(SubmitNode {
            sort_key,
            distance,
            data,
        }) as SubmitNodeId
    }

    pub fn get_submit_node_data(
        &self,
        index: SubmitNodeId,
    ) -> &SubmitNode<SubmitPacketDataT::SubmitNodeData> {
        self.submit_nodes.get(index as usize)
    }

    pub fn is_relevant(
        &self,
        render_phase: RenderPhaseIndex,
    ) -> bool {
        self.render_phase == render_phase
    }
}

impl<SubmitPacketDataT: 'static + Sync + Send + SubmitPacketData> RenderFeatureSubmitNodeBlock
    for SubmitNodeBlock<SubmitPacketDataT>
{
    fn render_phase(&self) -> RenderPhaseIndex {
        self.render_phase
    }

    fn num_submit_nodes(&self) -> usize {
        self.len()
    }

    fn get_submit_node(
        &self,
        submit_node_id: SubmitNodeId,
    ) -> RenderFeatureSubmitNode {
        let submit_node = self.get_submit_node_data(submit_node_id);
        RenderFeatureSubmitNode::new(
            self.feature_index,
            submit_node_id,
            submit_node.sort_key,
            submit_node.distance,
        )
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        self.feature_index
    }
}

/// Each `SubmitNode` contains the data needed for the `RenderFeature`'s `RenderFeatureWriteJob` to
/// render a draw call by referencing data in the frame packet, submit packet, render objects set, or
/// some other storage. `SubmitNode`s will be sorted by the `RenderPhase` after they are combined into
/// a `ViewPhaseSubmitNodeBlock`.
pub struct SubmitNode<T> {
    pub sort_key: SubmitNodeSortKey,
    pub distance: f32,
    pub data: T,
}

impl<T: Default> SubmitNode<T> {
    pub fn new() -> Self {
        Self {
            sort_key: 0,
            distance: 0.,
            data: T::default(),
        }
    }
}
