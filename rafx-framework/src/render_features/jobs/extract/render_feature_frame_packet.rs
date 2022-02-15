use crate::render_features::render_features_prelude::*;
use downcast_rs::{impl_downcast, Downcast};

/// The `ID` of a `RenderObjectInstance` in a specific `FramePacket` for the current frame.
pub type RenderObjectInstanceId = u32;

impl_downcast!(RenderFeatureFramePacket);
/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`
/// to control the workload of the rendering process without identifying specific types
/// used in each `RenderFeature`'s frame packet or workload. See `FramePacket` and `ViewPacket`
/// for implementation details.
pub trait RenderFeatureFramePacket: Downcast + Send + Sync {
    fn get_or_push_render_object_instance(
        &mut self,
        render_object_instance: RenderObjectInstance,
    ) -> RenderObjectInstanceId;

    fn push_render_object_instance_per_view(
        &mut self,
        view_index: ViewFrameIndex,
        render_object_instance_id: RenderObjectInstanceId,
        render_object_instance: RenderObjectInstance,
    ) -> RenderObjectInstancePerViewId {
        let view_packet = self.render_feature_view_packet_mut(view_index);
        view_packet.push_render_object_instance(render_object_instance_id, render_object_instance)
    }

    fn push_volume(
        &mut self,
        view_index: ViewFrameIndex,
        object_id: ObjectId,
    ) {
        let view_packet = self.render_feature_view_packet_mut(view_index);
        view_packet.push_volume(object_id);
    }

    fn render_feature_view_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewPacket;

    fn render_feature_view_packet_mut(
        &mut self,
        view_index: ViewFrameIndex,
    ) -> &mut dyn RenderFeatureViewPacket;

    fn feature_index(&self) -> RenderFeatureIndex;
}

/// Provides `into_concrete` method to downcast into a concrete type.
pub trait RenderFeatureFramePacketIntoConcrete {
    /// Downcast `Box<dyn RenderFeatureFramePacket>` into `Box<T>` where `T: RenderFeatureFramePacket`.
    fn into_concrete<T: RenderFeatureFramePacket>(self) -> Box<T>;
}

impl RenderFeatureFramePacketIntoConcrete for Box<dyn RenderFeatureFramePacket> {
    fn into_concrete<T: RenderFeatureFramePacket>(self) -> Box<T> {
        self.into_any().downcast::<T>().unwrap_or_else(|_| {
            panic!(
                "Unable to downcast {} into {}",
                std::any::type_name::<dyn RenderFeatureFramePacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}

/// Provides `as_concrete` method to downcast as a concrete type.
pub trait RenderFeatureFramePacketAsConcrete<'a> {
    /// Downcast `&dyn RenderFeatureFramePacket` into `&T` where `T: RenderFeatureFramePacket`.
    fn as_concrete<T: RenderFeatureFramePacket>(&'a self) -> &'a T;
}

impl<'a> RenderFeatureFramePacketAsConcrete<'a> for dyn RenderFeatureFramePacket {
    fn as_concrete<T: RenderFeatureFramePacket>(&'a self) -> &'a T {
        self.as_any().downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "Unable to downcast_ref {} into {}",
                std::any::type_name::<dyn RenderFeatureFramePacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}
