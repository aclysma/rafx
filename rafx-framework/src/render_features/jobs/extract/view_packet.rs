use crate::render_features::render_features_prelude::*;

/// Read documentation on `FramePacketData`.
pub struct ViewPacket<FramePacketDataT: FramePacketData> {
    view: RenderView,
    view_frame_index: ViewFrameIndex,

    pub(crate) per_view_data: AtomicOnceCell<FramePacketDataT::PerViewData>,
    pub(crate) render_object_instances: Vec<RenderObjectInstancePerView>,
    pub(crate) render_object_instances_data:
        AtomicOnceCellArray<FramePacketDataT::RenderObjectInstancePerViewData>,

    volumes: Vec<ObjectId>,
}

impl<FramePacketDataT: FramePacketData> ViewPacket<FramePacketDataT> {
    pub fn new(
        view_packet_size: &ViewPacketSize,
        view_frame_index: ViewFrameIndex,
    ) -> Self {
        Self {
            view: view_packet_size.view.clone(),
            view_frame_index,
            per_view_data: AtomicOnceCell::new(),
            render_object_instances: Vec::with_capacity(
                view_packet_size.num_render_object_instances,
            ),
            render_object_instances_data: AtomicOnceCellArray::with_capacity(
                view_packet_size.num_render_object_instances,
            ),
            volumes: Vec::with_capacity(view_packet_size.num_volumes),
        }
    }

    pub fn render_object_instances(&self) -> &Vec<RenderObjectInstancePerView> {
        &self.render_object_instances
    }

    pub fn render_object_instances_data(
        &self
    ) -> &AtomicOnceCellArray<FramePacketDataT::RenderObjectInstancePerViewData> {
        &self.render_object_instances_data
    }

    pub fn per_view_data(&self) -> &AtomicOnceCell<FramePacketDataT::PerViewData> {
        &self.per_view_data
    }

    pub fn view_frame_index(&self) -> ViewFrameIndex {
        self.view_frame_index
    }

    pub fn volumes(&self) -> &Vec<ObjectId> {
        &self.volumes
    }
}

impl<FramePacketDataT: 'static + FramePacketData> RenderFeatureViewPacket
    for ViewPacket<FramePacketDataT>
{
    fn view(&self) -> &RenderView {
        &self.view
    }

    fn view_frame_index(&self) -> ViewFrameIndex {
        self.view_frame_index
    }

    fn num_render_object_instances(&self) -> usize {
        self.render_object_instances.len()
    }

    fn push_render_object_instance(
        &mut self,
        render_object_instance_id: RenderObjectInstanceId,
        render_object_instance: RenderObjectInstance,
    ) -> RenderObjectInstancePerViewId {
        let index = self.render_object_instances.len();
        self.render_object_instances
            .push(RenderObjectInstancePerView::new(
                render_object_instance_id,
                render_object_instance,
            ));
        index as RenderObjectInstancePerViewId
    }

    fn push_volume(
        &mut self,
        object_id: ObjectId,
    ) {
        self.volumes.push(object_id);
    }
}

/// A specific `RenderObjectInstance` as viewed by some `RenderView`.
#[derive(Copy, Eq, PartialEq, Hash, Clone, Debug)]
pub struct RenderObjectInstancePerView {
    pub render_object_instance_id: RenderObjectInstanceId,
    pub render_object_instance: RenderObjectInstance,
}

impl RenderObjectInstancePerView {
    pub fn new(
        render_object_instance_id: RenderObjectInstanceId,
        render_object_instance: RenderObjectInstance,
    ) -> Self {
        Self {
            render_object_instance_id,
            render_object_instance,
        }
    }
}
