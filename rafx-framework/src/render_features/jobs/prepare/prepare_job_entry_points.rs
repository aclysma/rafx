use crate::render_features::render_features_prelude::*;

/// `PrepareJobEntryPoints` provides a generic set of callbacks for a `RenderFeature`
/// compatible with the `PrepareJob` struct. This simplifies the work of implementing
/// the `RenderFeaturePrepareJob` trait.
pub trait PrepareJobEntryPoints<'prepare>: Sync + Send + Sized {
    /// Called once at the start of the `prepare` step when any `RenderView` in the frame is
    /// relevant to this `RenderFeature`.
    fn begin_per_frame_prepare(
        &self,
        _context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
    }

    /// Called once for each instance of an `Entity` and `RenderObject` in the frame matching this
    /// `RenderFeature`.
    fn prepare_render_object_instance(
        &self,
        _job_context: &mut Self::RenderObjectInstanceJobContextT,
        _context: &PrepareRenderObjectInstanceContext<'prepare, '_, Self>,
    ) {
    }

    /// Called once for each instance of an `Entity` and `RenderObject` in each `RenderView` relevant
    /// to this `RenderFeature`.
    fn prepare_render_object_instance_per_view(
        &self,
        _job_context: &mut Self::RenderObjectInstancePerViewJobContextT,
        _context: &PrepareRenderObjectInstancePerViewContext<'prepare, '_, Self>,
    ) {
    }

    /// Called once for each relevant `RenderView`. This function is only run after all instances of
    /// `prepare_render_object_instance_per_view` have finished for that `RenderView`.
    fn end_per_view_prepare(
        &self,
        _context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
    }

    /// Called once at the end of the `prepare` step when any `RenderView` in the frame is
    /// relevant to this `RenderFeature`.
    fn end_per_frame_prepare(
        &self,
        _context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants;

    fn feature_index(&self) -> RenderFeatureIndex;

    fn new_render_object_instance_job_context(
        &'prepare self
    ) -> Option<Self::RenderObjectInstanceJobContextT> {
        None
    }

    fn new_render_object_instance_per_view_job_context(
        &'prepare self
    ) -> Option<Self::RenderObjectInstancePerViewJobContextT> {
        None
    }

    /// `JobContext` for the `prepare_render_object_instance` entry point.
    type RenderObjectInstanceJobContextT;

    /// `JobContext` for the `prepare_render_object_instance_per_view` entry point.
    type RenderObjectInstancePerViewJobContextT;

    type FramePacketDataT: 'static + Sync + Send + FramePacketData;

    type SubmitPacketDataT: 'static + Sync + Send + SubmitPacketData;
}

pub struct PreparePerFrameContext<
    'prepare,
    'entry,
    PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>,
> {
    frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
    submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
}

impl<'prepare, 'entry, PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>>
    PreparePerFrameContext<'prepare, 'entry, PrepareJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
        submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
    ) -> Self {
        Self {
            frame_packet,
            submit_packet,
        }
    }

    pub fn per_frame_data(
        &self
    ) -> &<PrepareJobEntryPointsT::FramePacketDataT as FramePacketData>::PerFrameData {
        &self.frame_packet.per_frame_data.get()
    }

    pub fn per_frame_submit_data(
        &self
    ) -> &<PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::PerFrameSubmitData {
        &self.submit_packet.per_frame_submit_data().get()
    }

    pub fn frame_packet(&self) -> &FramePacket<PrepareJobEntryPointsT::FramePacketDataT> {
        self.frame_packet
    }

    pub fn submit_packet(&self) -> &SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT> {
        self.submit_packet
    }
}

pub struct PrepareRenderObjectInstanceContext<
    'prepare,
    'entry,
    PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>,
> {
    frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
    submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
    render_object_instance: &'entry RenderObjectInstance,
    id: usize,
}

impl<'prepare, 'entry, PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>>
    PrepareRenderObjectInstanceContext<'prepare, 'entry, PrepareJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
        submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
        id: usize,
    ) -> Self {
        let render_object_instance = &frame_packet.render_object_instances[id];
        Self {
            frame_packet,
            submit_packet,
            render_object_instance,
            id,
        }
    }

    pub fn render_object_instance_data(
        &self
    ) -> &<PrepareJobEntryPointsT::FramePacketDataT as FramePacketData>::RenderObjectInstanceData
    {
        self.frame_packet.render_object_instances_data.get(self.id)
    }

    pub fn object_id(&self) -> ObjectId {
        self.render_object_instance.object_id
    }

    pub fn render_object_id(&self) -> &RenderObjectId {
        &self.render_object_instance.render_object_id
    }

    pub fn set_render_object_instance_submit_data(
        &self,
        data: <PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::RenderObjectInstanceSubmitData,
    ) {
        self.submit_packet
            .render_object_instances_submit_data
            .set(self.id, data);
    }
}

pub struct PreparePerViewContext<
    'prepare,
    'entry,
    PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>,
> {
    frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
    submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
    view_packet: &'entry ViewPacket<PrepareJobEntryPointsT::FramePacketDataT>,
    view_submit_packet: &'entry ViewSubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
}

impl<'prepare, 'entry, PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>>
    PreparePerViewContext<'prepare, 'entry, PrepareJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
        submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
        view_packet: &'entry ViewPacket<PrepareJobEntryPointsT::FramePacketDataT>,
        view_submit_packet: &'entry ViewSubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
    ) -> Self {
        Self {
            frame_packet,
            submit_packet,
            view_packet,
            view_submit_packet,
        }
    }

    pub fn per_frame_data(
        &self
    ) -> &<PrepareJobEntryPointsT::FramePacketDataT as FramePacketData>::PerFrameData {
        &self.frame_packet.per_frame_data.get()
    }

    pub fn per_view_data(
        &self
    ) -> &<PrepareJobEntryPointsT::FramePacketDataT as FramePacketData>::PerViewData {
        &self.view_packet.per_view_data.get()
    }

    pub fn per_frame_submit_data(
        &self
    ) -> &<PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::PerFrameSubmitData {
        &self.submit_packet.per_frame_submit_data.get()
    }

    pub fn render_object_instances(&self) -> &[RenderObjectInstance] {
        &self.frame_packet.render_object_instances
    }

    // TODO(dvd): This should use an `as_slice` method on the data.
    pub fn render_object_instances_data(
        &self
    ) -> &AtomicOnceCellArray<
        <PrepareJobEntryPointsT::FramePacketDataT as FramePacketData>::RenderObjectInstanceData,
    > {
        &self.frame_packet.render_object_instances_data
    }

    pub fn view(&self) -> &RenderView {
        self.view_packet.view()
    }

    pub fn view_packet(&self) -> &ViewPacket<PrepareJobEntryPointsT::FramePacketDataT> {
        &self.view_packet
    }

    pub fn view_submit_packet(
        &self
    ) -> &ViewSubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT> {
        &self.view_submit_packet
    }
}

pub struct PrepareRenderObjectInstancePerViewContext<
    'prepare,
    'entry,
    PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>,
> {
    frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
    submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
    view_packet: &'entry ViewPacket<PrepareJobEntryPointsT::FramePacketDataT>,
    view_submit_packet: &'entry ViewSubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
    render_object_instance_per_view: &'entry RenderObjectInstancePerView,
    id: usize,
}

impl<'prepare, 'entry, PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>>
    PrepareRenderObjectInstancePerViewContext<'prepare, 'entry, PrepareJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<PrepareJobEntryPointsT::FramePacketDataT>,
        submit_packet: &'entry SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
        view_packet: &'entry ViewPacket<PrepareJobEntryPointsT::FramePacketDataT>,
        view_submit_packet: &'entry ViewSubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>,
        id: usize,
    ) -> Self {
        let render_object_instance_per_view = &view_packet.render_object_instances[id];
        Self {
            frame_packet,
            submit_packet,
            view_packet,
            view_submit_packet,
            render_object_instance_per_view,
            id,
        }
    }

    pub fn render_object_instance_id(&self) -> RenderObjectInstanceId {
        self.render_object_instance_per_view
            .render_object_instance_id
    }

    pub fn object_id(&self) -> ObjectId {
        self.render_object_instance_per_view
            .render_object_instance
            .object_id
    }

    pub fn render_object_id(&self) -> &RenderObjectId {
        &self
            .render_object_instance_per_view
            .render_object_instance
            .render_object_id
    }

    pub fn render_object_instance_data(
        &self
    ) -> &<PrepareJobEntryPointsT::FramePacketDataT as FramePacketData>::RenderObjectInstanceData
    {
        self.frame_packet.render_object_instances_data.get(
            self.render_object_instance_per_view
                .render_object_instance_id as usize,
        )
    }

    pub fn render_object_instance_submit_data(
        &self
    ) -> &<PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::RenderObjectInstanceSubmitData
    {
        self.submit_packet.render_object_instances_submit_data.get(
            self.render_object_instance_per_view
                .render_object_instance_id as usize,
        )
    }

    pub fn view(&self) -> &RenderView {
        self.view_packet.view()
    }

    pub fn set_render_object_instance_per_view_submit_data(
        &self,
        data: <PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::RenderObjectInstancePerViewSubmitData,
    ) {
        self.view_submit_packet
            .render_object_instances_submit_data
            .set(self.id, data);
    }

    pub fn push_submit_node<RenderPhaseT: RenderPhase>(
        &self,
        data: <PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::SubmitNodeData,
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
        data: <PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::SubmitNodeData,
        sort_key: SubmitNodeSortKey,
        distance: f32,
    ) -> SubmitNodeId {
        self.view_submit_packet.push_submit_node_into_render_phase(
            render_phase,
            data,
            sort_key,
            distance,
        )
    }

    pub fn get_submit_node_data<RenderPhaseT: RenderPhase>(
        &self,
        index: SubmitNodeId,
    ) -> &<PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::SubmitNodeData {
        self.get_submit_node_data_from_render_phase(RenderPhaseT::render_phase_index(), index)
    }

    pub fn get_submit_node_data_from_render_phase(
        &self,
        render_phase: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> &<PrepareJobEntryPointsT::SubmitPacketDataT as SubmitPacketData>::SubmitNodeData {
        self.view_submit_packet
            .get_submit_node_data_from_render_phase(render_phase, index)
    }
}
