use crate::render_features::render_features_prelude::*;
use std::marker::PhantomData;
use std::ops::Range;

/// A `PrepareJob` implements the `RenderFeaturePrepareJob` trait by wrapping an instance
/// of an `PrepareJobEntryPoints` type defined at compile-time. The `PrepareJob` contains
/// the frame & submit packets and presents the correct context (like `PreparePerFrameContext`)
/// to each entry point defined in `RenderFeaturePrepareJob`.
pub struct PrepareJob<'prepare, PrepareJobEntryPointsT: PrepareJobEntryPoints<'prepare>> {
    inner: PrepareJobEntryPointsT,
    frame_packet: Option<Box<FramePacket<PrepareJobEntryPointsT::FramePacketDataT>>>,
    submit_packet: Option<Box<SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>>>,
    #[allow(dead_code)]
    debug_constants: &'static RenderFeatureDebugConstants,
    _phantom: (PhantomData<&'prepare ()>,),
}

impl<'prepare, PrepareJobEntryPointsT: 'prepare + PrepareJobEntryPoints<'prepare>>
    PrepareJob<'prepare, PrepareJobEntryPointsT>
{
    pub fn new(
        inner: PrepareJobEntryPointsT,
        frame_packet: Box<FramePacket<PrepareJobEntryPointsT::FramePacketDataT>>,
        submit_packet: Box<SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>>,
    ) -> Self {
        let debug_constants = inner.feature_debug_constants();
        Self {
            inner,
            frame_packet: Some(frame_packet),
            submit_packet: Some(submit_packet),
            debug_constants,
            _phantom: Default::default(),
        }
    }

    fn frame_packet(&self) -> &Option<Box<FramePacket<PrepareJobEntryPointsT::FramePacketDataT>>> {
        &self.frame_packet
    }

    fn view_packets(&self) -> &Vec<ViewPacket<PrepareJobEntryPointsT::FramePacketDataT>> {
        &self.frame_packet.as_ref().unwrap().view_packets()
    }

    fn render_object_instances(&self) -> &Vec<RenderObjectInstance> {
        &self.frame_packet.as_ref().unwrap().render_object_instances
    }

    fn submit_packet(
        &self
    ) -> &Option<Box<SubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT>>> {
        &self.submit_packet
    }

    fn force_to_prepare_lifetime(
        &self,
        inner: &PrepareJobEntryPointsT,
    ) -> &'prepare PrepareJobEntryPointsT {
        unsafe {
            // SAFETY: The 'prepare lifetime added here is already required by the PrepareJobEntryPointsT.
            // This transmute is just avoiding the need to proliferate even _more_ 'prepare lifetimes through
            // _every single function_.
            std::mem::transmute::<_, &'prepare PrepareJobEntryPointsT>(inner)
        }
    }
}

impl<'prepare, PrepareJobEntryPointsT: 'prepare + PrepareJobEntryPoints<'prepare>>
    RenderFeaturePrepareJob<'prepare> for PrepareJob<'prepare, PrepareJobEntryPointsT>
{
    fn begin_per_frame_prepare(&self) {
        profiling::scope!(self.debug_constants.begin_per_frame_prepare);

        let context = PreparePerFrameContext::new(
            self.frame_packet.as_ref().unwrap(),
            self.submit_packet.as_ref().unwrap(),
        );
        self.inner.begin_per_frame_prepare(&context);
    }

    fn prepare_render_object_instance(
        &self,
        range: Range<usize>,
    ) {
        if range.is_empty() {
            return;
        }

        let mut job_context = {
            let inner = self.force_to_prepare_lifetime(&self.inner);
            inner.new_render_object_instance_job_context()
        };

        if job_context.is_none() {
            return;
        }

        profiling::scope!(self.debug_constants.prepare_render_object_instance);

        let job_context = job_context.as_mut().unwrap();
        let frame_packet = self.frame_packet.as_ref().unwrap();
        let submit_packet = self.submit_packet.as_ref().unwrap();
        for id in range {
            let context = PrepareRenderObjectInstanceContext::new(frame_packet, submit_packet, id);
            self.inner
                .prepare_render_object_instance(job_context, &context);
        }
    }

    fn view_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewPacket {
        self.frame_packet()
            .as_ref()
            .unwrap()
            .render_feature_view_packet(view_index)
    }

    fn view_submit_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewSubmitPacket {
        self.submit_packet()
            .as_ref()
            .unwrap()
            .render_feature_view_submit_packet(view_index)
    }

    fn prepare_render_object_instance_per_view(
        &self,
        view_packet: &dyn RenderFeatureViewPacket,
        view_submit_packet: &dyn RenderFeatureViewSubmitPacket,
        range: Range<usize>,
    ) {
        if range.is_empty() {
            return;
        }

        let mut job_context = {
            let inner = self.force_to_prepare_lifetime(&self.inner);
            inner.new_render_object_instance_per_view_job_context()
        };

        if job_context.is_none() {
            return;
        }

        profiling::scope!(self.debug_constants.prepare_render_object_instance_per_view);

        let job_context = job_context.as_mut().unwrap();
        let frame_packet = self.frame_packet.as_ref().unwrap();
        let submit_packet = self.submit_packet.as_ref().unwrap();

        let view_packet: &ViewPacket<PrepareJobEntryPointsT::FramePacketDataT> =
            view_packet.as_concrete();

        let view_submit_packet: &ViewSubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT> =
            view_submit_packet.as_concrete();

        for id in range {
            let context = PrepareRenderObjectInstancePerViewContext::new(
                frame_packet,
                submit_packet,
                view_packet,
                view_submit_packet,
                id,
            );
            self.inner
                .prepare_render_object_instance_per_view(job_context, &context);
        }
    }

    fn end_per_view_prepare(
        &self,
        view_packet: &dyn RenderFeatureViewPacket,
        view_submit_packet: &dyn RenderFeatureViewSubmitPacket,
    ) {
        profiling::scope!(self.debug_constants.end_per_view_prepare);

        let view_packet: &ViewPacket<PrepareJobEntryPointsT::FramePacketDataT> =
            view_packet.as_concrete();

        let view_submit_packet: &ViewSubmitPacket<PrepareJobEntryPointsT::SubmitPacketDataT> =
            view_submit_packet.as_concrete();

        let context = PreparePerViewContext::new(
            self.frame_packet.as_ref().unwrap(),
            self.submit_packet.as_ref().unwrap(),
            view_packet,
            view_submit_packet,
        );

        self.inner.end_per_view_prepare(&context);
    }

    fn end_per_frame_prepare(&self) {
        profiling::scope!(self.debug_constants.end_per_frame_prepare);

        let context = PreparePerFrameContext::new(
            self.frame_packet.as_ref().unwrap(),
            self.submit_packet.as_ref().unwrap(),
        );
        self.inner.end_per_frame_prepare(&context);
    }

    fn num_views(&self) -> usize {
        self.view_packets().len()
    }

    fn num_render_object_instances(&self) -> usize {
        self.render_object_instances().len()
    }

    fn take_frame_packet(&mut self) -> Box<dyn RenderFeatureFramePacket> {
        std::mem::take(&mut self.frame_packet).unwrap()
    }

    fn take_submit_packet(&mut self) -> Box<dyn RenderFeatureSubmitPacket> {
        std::mem::take(&mut self.submit_packet).unwrap()
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        self.inner.feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        self.inner.feature_index()
    }
}
