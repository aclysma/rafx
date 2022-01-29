use crate::render_features::render_features_prelude::*;

/// The `SubmitPacket` is the data that must be prepared from
/// in order for the `RenderFeature`'s `WriteJob` to create each
/// draw call. The draw calls may reference data in either the
/// `FramePacket` or the `SubmitPacket`. Each draw call is represented
/// by exactly 1 `SubmitNode`. In order to allocate the `SubmitPacket`,
/// the `RenderFeature` is given a reference to the populated `FramePacket`
/// from that frame. The `RenderFeature` **must** size the `SubmitPacket`
/// appropriately. The `SubmitPacket` **must** be considered
/// immutable after the `Prepare` step has finished.
pub trait SubmitPacketData {
    /// All data that is either unassociated with or shared by
    /// any of the submit nodes across each `RenderView`.
    type PerFrameSubmitData: Sync + Send;

    /// All data for any submit nodes associated with a particular
    /// `Entity` and `RenderObject`.
    type RenderObjectInstanceSubmitData: Sync + Send;

    /// All data that can be shared by the submit nodes for
    /// this `RenderFeature`'s `WriteJob` in this `RenderView`
    /// and `RenderPhase`.
    type PerViewSubmitData: Sync + Send;

    /// All data for any submit nodes associated with a particular
    /// `Entity`, `RenderObject`, and `RenderView`.
    type RenderObjectInstancePerViewSubmitData: Sync + Send;

    /// The data needed by this `RenderFeature`'s `WriteJob` for
    /// each draw call in this `RenderView` and `RenderPhase` .
    type SubmitNodeData: Sync + Send;

    /// The `RenderFeature` associated with the `SubmitNodeBlock`s.
    /// This is used to find the correct `WriteJob` when writing
    /// the `PreparedRenderData`.
    type RenderFeature: RenderFeature;

    // TODO(dvd): see issue #29661 <https://github.com/rust-lang/rust/issues/29661> for more information
    // type SubmitPacket = SubmitPacket<Self>;
}

/// Read documentation on `SubmitPacketData`.
pub struct SubmitPacket<SubmitPacketDataT: SubmitPacketData> {
    feature_index: RenderFeatureIndex,

    pub(crate) per_frame_submit_data: AtomicOnceCell<SubmitPacketDataT::PerFrameSubmitData>,
    pub(crate) render_object_instances_submit_data:
        AtomicOnceCellArray<SubmitPacketDataT::RenderObjectInstanceSubmitData>,

    view_submit_packets: Vec<ViewSubmitPacket<SubmitPacketDataT>>,
}

impl<SubmitPacketDataT: SubmitPacketData> SubmitPacket<SubmitPacketDataT> {
    pub fn new(
        feature_index: RenderFeatureIndex,
        num_render_object_instances: usize,
        view_submit_packets: Vec<ViewSubmitPacket<SubmitPacketDataT>>,
    ) -> Self {
        Self {
            feature_index,
            per_frame_submit_data: AtomicOnceCell::new(),
            render_object_instances_submit_data: AtomicOnceCellArray::with_capacity(
                num_render_object_instances,
            ),
            view_submit_packets,
        }
    }

    pub fn view_submit_packets(&self) -> &Vec<ViewSubmitPacket<SubmitPacketDataT>> {
        &self.view_submit_packets
    }

    pub fn view_submit_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &ViewSubmitPacket<SubmitPacketDataT> {
        self.view_submit_packets
            .get(view_index as usize)
            .unwrap_or_else(|| {
                panic!(
                    "ViewSubmitPacket with ViewFrameIndex {} was not found in {}.",
                    view_index,
                    std::any::type_name::<SubmitPacketDataT>()
                )
            })
    }

    pub fn render_object_instances_submit_data(
        &self
    ) -> &AtomicOnceCellArray<SubmitPacketDataT::RenderObjectInstanceSubmitData> {
        &self.render_object_instances_submit_data
    }

    pub fn per_frame_submit_data(&self) -> &AtomicOnceCell<SubmitPacketDataT::PerFrameSubmitData> {
        &self.per_frame_submit_data
    }
}

impl<SubmitPacketDataT: 'static + Send + Sync + SubmitPacketData> RenderFeatureSubmitPacket
    for SubmitPacket<SubmitPacketDataT>
{
    fn render_feature_view_submit_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewSubmitPacket {
        self.view_submit_packet(view_index)
    }

    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex {
        self.view_submit_packets
            .iter()
            .position(|view_submit_packet| {
                view_submit_packet.view().view_index() == view.view_index()
            })
            .unwrap_or_else(|| {
                panic!(
                    "View {} with ViewIndex {} was not found in {}.",
                    view.debug_name(),
                    view.view_index(),
                    std::any::type_name::<SubmitPacketDataT>()
                )
            }) as ViewFrameIndex
    }

    fn view_frame_index_from_view_index(
        &self,
        view_index: RenderViewIndex,
    ) -> Option<ViewFrameIndex> {
        self.view_submit_packets
            .iter()
            .position(|view_submit_packet| view_submit_packet.view().view_index() == view_index)
            .map(|x| x as ViewFrameIndex)
    }

    fn feature_index(&self) -> u32 {
        self.feature_index
    }
}
