use crate::render_features::registry::MAX_RENDER_FEATURE_COUNT;
use crate::render_features::render_features_prelude::*;

/// Read documentation on `SubmitPacketData`.
pub struct ViewSubmitPacket<SubmitPacketDataT: SubmitPacketData> {
    view: RenderView,

    pub(crate) per_view_submit_data: AtomicOnceCell<SubmitPacketDataT::PerViewSubmitData>,
    pub(crate) render_object_instances_submit_data:
        AtomicOnceCellArray<SubmitPacketDataT::RenderObjectInstancePerViewSubmitData>,

    submit_node_blocks: Vec<SubmitNodeBlock<SubmitPacketDataT>>,
    submit_node_phases: [Option<u8>; MAX_RENDER_FEATURE_COUNT as usize],
}

impl<SubmitPacketDataT: 'static + Send + Sync + SubmitPacketData>
    ViewSubmitPacket<SubmitPacketDataT>
{
    pub fn from_view_packet<RenderPhaseT: RenderPhase>(
        view_packet: &dyn RenderFeatureViewPacket,
        num_submit_nodes: Option<usize>,
    ) -> Self {
        let view_packet_size = ViewPacketSize::size_of(view_packet);
        let submit_node_blocks = vec![SubmitNodeBlock::with_capacity::<RenderPhaseT>(
            view_packet.view(),
            num_submit_nodes.unwrap_or(view_packet_size.num_render_object_instances),
        )];

        ViewSubmitPacket::new(submit_node_blocks, &view_packet_size)
    }

    pub fn new(
        submit_node_blocks: Vec<SubmitNodeBlock<SubmitPacketDataT>>,
        view_packet_size: &ViewPacketSize,
    ) -> Self {
        assert!((u8::MAX as u32) > MAX_RENDER_FEATURE_COUNT);
        let mut submit_node_phases = [None; MAX_RENDER_FEATURE_COUNT as usize];
        for (index, submit_node_block) in submit_node_blocks.iter().enumerate() {
            submit_node_phases[submit_node_block.render_phase() as usize] = Some(index as u8);
        }

        Self {
            view: view_packet_size.view.clone(),
            per_view_submit_data: AtomicOnceCell::new(),
            render_object_instances_submit_data: AtomicOnceCellArray::with_capacity(
                view_packet_size.num_render_object_instances,
            ),
            submit_node_phases,
            submit_node_blocks,
        }
    }

    pub fn render_object_instances_submit_data(
        &self
    ) -> &AtomicOnceCellArray<SubmitPacketDataT::RenderObjectInstancePerViewSubmitData> {
        &self.render_object_instances_submit_data
    }

    pub fn per_view_submit_data(&self) -> &AtomicOnceCell<SubmitPacketDataT::PerViewSubmitData> {
        &self.per_view_submit_data
    }

    pub fn push_submit_node<RenderPhaseT: RenderPhase>(
        &self,
        data: SubmitPacketDataT::SubmitNodeData,
        sort_key: SubmitNodeSortKey,
        distance: f32,
    ) -> SubmitNodeId {
        self.push_submit_node_into_render_phase(
            RenderPhaseT::render_phase_index(),
            data,
            sort_key,
            distance,
        )
    }

    pub fn push_submit_node_into_render_phase(
        &self,
        render_phase: RenderPhaseIndex,
        data: SubmitPacketDataT::SubmitNodeData,
        sort_key: SubmitNodeSortKey,
        distance: f32,
    ) -> SubmitNodeId {
        self.submit_node_block(render_phase)
            .push_submit_node(data, sort_key, distance)
    }

    fn submit_node_block(
        &self,
        render_phase: RenderPhaseIndex,
    ) -> &SubmitNodeBlock<SubmitPacketDataT> {
        self.submit_node_phases[render_phase as usize]
            .map(|index| &self.submit_node_blocks[index as usize])
            .unwrap_or_else(|| {
                panic!(
                    "{} does not contain RenderPhase {}",
                    std::any::type_name::<ViewSubmitPacket<SubmitPacketDataT>>(),
                    render_phase
                )
            })
    }

    pub fn get_submit_node_data<RenderPhaseT: RenderPhase>(
        &self,
        index: SubmitNodeId,
    ) -> &SubmitPacketDataT::SubmitNodeData {
        self.get_submit_node_data_from_render_phase(RenderPhaseT::render_phase_index(), index)
    }

    pub fn get_submit_node_data_from_render_phase(
        &self,
        render_phase: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> &SubmitPacketDataT::SubmitNodeData {
        &self
            .submit_node_block(render_phase)
            .get_submit_node_data(index)
            .data
    }
}

impl<SubmitPacketDataT: 'static + Send + Sync + SubmitPacketData> RenderFeatureViewSubmitPacket
    for ViewSubmitPacket<SubmitPacketDataT>
{
    fn view(&self) -> &RenderView {
        &self.view
    }

    fn num_submit_nodes(
        &self,
        render_phase: RenderPhaseIndex,
    ) -> usize {
        self.submit_node_phases[render_phase as usize]
            .map(|index| self.submit_node_blocks[index as usize].num_submit_nodes())
            .unwrap_or(0)
    }

    fn get_submit_node_block(
        &self,
        render_phase: RenderPhaseIndex,
    ) -> Option<&dyn RenderFeatureSubmitNodeBlock> {
        self.submit_node_phases[render_phase as usize].map(|index| {
            let submit_node_block: &dyn RenderFeatureSubmitNodeBlock =
                &self.submit_node_blocks[index as usize];
            submit_node_block
        })
    }
}
