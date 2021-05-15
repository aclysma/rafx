use crate::render_features::render_features_prelude::*;
use downcast_rs::{impl_downcast, Downcast};

impl_downcast!(RenderFeatureSubmitPacket);
/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`
/// to control the workload of the rendering process without identifying specific types
/// used in each `RenderFeature`'s frame packet or workload. See `SubmitPacket` and `ViewSubmitPacket`
/// for implementation details.
pub trait RenderFeatureSubmitPacket: Downcast + Send + Sync {
    fn render_feature_view_submit_packet(
        &self,
        view_index: ViewFrameIndex,
    ) -> &dyn RenderFeatureViewSubmitPacket;

    fn view_frame_index(
        &self,
        view: &RenderView,
    ) -> ViewFrameIndex;

    fn feature_index(&self) -> RenderFeatureIndex;
}

/// Provides `into_concrete` method to downcast into a concrete type.
pub trait RenderFeatureSubmitPacketIntoConcrete {
    /// Downcast `Box<dyn RenderFeatureSubmitPacket>` into `Box<T>` where `T: RenderFeatureSubmitPacket`.
    fn into_concrete<T: RenderFeatureSubmitPacket>(self) -> Box<T>;
}

impl RenderFeatureSubmitPacketIntoConcrete for Box<dyn RenderFeatureSubmitPacket> {
    fn into_concrete<T: RenderFeatureSubmitPacket>(self) -> Box<T> {
        self.into_any().downcast::<T>().unwrap_or_else(|_| {
            panic!(
                "Unable to downcast {} into {}",
                std::any::type_name::<dyn RenderFeatureSubmitPacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}

/// Provides `as_concrete` method to downcast as a concrete type.
pub trait RenderFeatureSubmitPacketAsConcrete<'a> {
    /// Downcast `&dyn RenderFeatureSubmitPacket` into `&T` where `T: RenderFeatureSubmitPacket`.
    fn as_concrete<T: RenderFeatureSubmitPacket>(&'a self) -> &'a T;
}

impl<'a> RenderFeatureSubmitPacketAsConcrete<'a> for dyn RenderFeatureSubmitPacket {
    fn as_concrete<T: RenderFeatureSubmitPacket>(&'a self) -> &'a T {
        self.as_any().downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "Unable to downcast_ref {} into {}",
                std::any::type_name::<dyn RenderFeatureSubmitPacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}
