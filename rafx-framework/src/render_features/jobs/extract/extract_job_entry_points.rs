use crate::render_features::render_features_prelude::*;
use crate::render_features::RenderObjectInstancePerView;

/// `ExtractJobEntryPoints` provides a generic set of callbacks for a `RenderFeature`
/// compatible with the `ExtractJob` struct. This simplifies the work of implementing
/// the `RenderFeatureExtractJob` trait.
pub trait ExtractJobEntryPoints<'extract>: Sync + Send + Sized {
    /// Called once at the start of the `extract` step when any `RenderView` in the frame is
    /// relevant to this `RenderFeature`.
    fn begin_per_frame_extract(
        &self,
        _context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
    }

    /// Called once for each instance of an `Entity` and `RenderObject` in the frame matching this
    /// `RenderFeature`.
    fn extract_render_object_instance(
        &self,
        _job_context: &mut Self::RenderObjectInstanceJobContextT,
        _context: &ExtractRenderObjectInstanceContext<'extract, '_, Self>,
    ) {
    }

    /// Called once for each instance of an `Entity` and `RenderObject` in each `RenderView` relevant
    /// to this `RenderFeature`.
    fn extract_render_object_instance_per_view(
        &self,
        _job_context: &mut Self::RenderObjectInstancePerViewJobContextT,
        _context: &ExtractRenderObjectInstancePerViewContext<'extract, '_, Self>,
    ) {
    }

    /// Called once for each relevant `RenderView`. This function is only run after all instances of
    /// `extract_render_object_instance_per_view` have finished for that `RenderView`.
    fn end_per_view_extract(
        &self,
        _context: &ExtractPerViewContext<'extract, '_, Self>,
    ) {
    }

    /// Called once at the end of the `extract` step when any `RenderView` in the frame is
    /// relevant to this `RenderFeature`.
    fn end_per_frame_extract(
        &self,
        _context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants;

    fn feature_index(&self) -> RenderFeatureIndex;

    fn new_render_object_instance_job_context(
        &'extract self
    ) -> Option<Self::RenderObjectInstanceJobContextT> {
        None
    }

    fn new_render_object_instance_per_view_job_context(
        &'extract self
    ) -> Option<Self::RenderObjectInstancePerViewJobContextT> {
        None
    }

    /// `JobContext` for the `extract_render_object_instance` entry point.
    type RenderObjectInstanceJobContextT;

    /// `JobContext` for the `extract_render_object_instance_per_view` entry point.
    type RenderObjectInstancePerViewJobContextT;

    /// See definition of `FramePacketData`.
    type FramePacketDataT: 'static + Sync + Send + FramePacketData;
}

pub struct ExtractPerFrameContext<
    'extract,
    'entry,
    ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>,
> {
    frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>,
}

impl<'extract, 'entry, ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>>
    ExtractPerFrameContext<'extract, 'entry, ExtractJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>
    ) -> Self {
        Self { frame_packet }
    }

    pub fn frame_packet(&self) -> &FramePacket<ExtractJobEntryPointsT::FramePacketDataT> {
        self.frame_packet
    }
}

pub struct ExtractRenderObjectInstanceContext<
    'extract,
    'entry,
    ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>,
> {
    frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>,
    render_object_instance: &'entry RenderObjectInstance,
    id: usize,
}

impl<'extract, 'entry, ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>>
    ExtractRenderObjectInstanceContext<'extract, 'entry, ExtractJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>,
        id: usize,
    ) -> Self {
        let render_object_instance = &frame_packet.render_object_instances[id];
        Self {
            frame_packet,
            render_object_instance,
            id,
        }
    }

    pub fn object_id(&self) -> ObjectId {
        self.render_object_instance.object_id
    }

    pub fn render_object_id(&self) -> &RenderObjectId {
        &self.render_object_instance.render_object_id
    }

    pub fn set_render_object_instance_data(
        &self,
        data: <ExtractJobEntryPointsT::FramePacketDataT as FramePacketData>::RenderObjectInstanceData,
    ) {
        self.frame_packet
            .render_object_instances_data
            .set(self.id, data);
    }
}

pub struct ExtractPerViewContext<
    'extract,
    'entry,
    ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>,
> {
    frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>,
    view_packet: &'entry ViewPacket<ExtractJobEntryPointsT::FramePacketDataT>,
}

impl<'extract, 'entry, ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>>
    ExtractPerViewContext<'extract, 'entry, ExtractJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>,
        view_packet: &'entry ViewPacket<ExtractJobEntryPointsT::FramePacketDataT>,
    ) -> Self {
        Self {
            frame_packet,
            view_packet,
        }
    }

    pub fn per_frame_data(
        &self
    ) -> &<ExtractJobEntryPointsT::FramePacketDataT as FramePacketData>::PerFrameData {
        &self.frame_packet.per_frame_data.get()
    }

    pub fn render_object_instances(&self) -> &[RenderObjectInstance] {
        &self.frame_packet.render_object_instances
    }

    // TODO(dvd): This should use an `as_slice` method on the data.
    pub fn render_object_instances_data(
        &self
    ) -> &AtomicOnceCellArray<
        <ExtractJobEntryPointsT::FramePacketDataT as FramePacketData>::RenderObjectInstanceData,
    > {
        &self.frame_packet.render_object_instances_data
    }

    pub fn view_packet(&self) -> &ViewPacket<ExtractJobEntryPointsT::FramePacketDataT> {
        &self.view_packet
    }

    pub fn view(&self) -> &RenderView {
        self.view_packet.view()
    }
}

pub struct ExtractRenderObjectInstancePerViewContext<
    'extract,
    'entry,
    ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>,
> {
    frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>,
    view_packet: &'entry ViewPacket<ExtractJobEntryPointsT::FramePacketDataT>,
    render_object_instance_per_view: &'entry RenderObjectInstancePerView,
    id: usize,
}

impl<'extract, 'entry, ExtractJobEntryPointsT: ExtractJobEntryPoints<'extract>>
    ExtractRenderObjectInstancePerViewContext<'extract, 'entry, ExtractJobEntryPointsT>
{
    pub fn new(
        frame_packet: &'entry FramePacket<ExtractJobEntryPointsT::FramePacketDataT>,
        view_packet: &'entry ViewPacket<ExtractJobEntryPointsT::FramePacketDataT>,
        id: usize,
    ) -> Self {
        let render_object_instance_per_view = &view_packet.render_object_instances[id];
        Self {
            frame_packet,
            view_packet,
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
    ) -> &<ExtractJobEntryPointsT::FramePacketDataT as FramePacketData>::RenderObjectInstanceData
    {
        self.frame_packet.render_object_instances_data.get(
            self.render_object_instance_per_view
                .render_object_instance_id as usize,
        )
    }

    pub fn view(&self) -> &RenderView {
        self.view_packet.view()
    }

    pub fn set_render_object_instance_per_view_data(
        &self,
        data: <ExtractJobEntryPointsT::FramePacketDataT as FramePacketData>::RenderObjectInstancePerViewData,
    ) {
        self.view_packet
            .render_object_instances_data
            .set(self.id, data);
    }
}
