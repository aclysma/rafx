use crate::render_features::render_features_prelude::*;
use downcast_rs::{impl_downcast, Downcast};

impl_downcast!(RenderFeatureViewSubmitPacket);
/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool` to control
/// the workload of the rendering process without identifying specific types used in each `RenderFeature`'s
/// frame packet or workload. See `ViewSubmitPacket` for implementation details.
pub trait RenderFeatureViewSubmitPacket: Downcast + Sync + Send {
    fn view(&self) -> &RenderView;

    fn num_submit_nodes(
        &self,
        render_phase: RenderPhaseIndex,
    ) -> usize;

    fn get_submit_node_block(
        &self,
        render_phase: RenderPhaseIndex,
    ) -> Option<&dyn RenderFeatureSubmitNodeBlock>;
}

/// Provides `into_concrete` method to downcast into a concrete type.
pub trait RenderFeatureViewSubmitPacketIntoConcrete {
    /// Downcast `Box<dyn RenderFeatureViewSubmitPacket>` into `Box<T>` where `T: RenderFeatureViewSubmitPacket`.
    fn into_concrete<T: RenderFeatureViewSubmitPacket>(self) -> Box<T>;
}

impl RenderFeatureViewSubmitPacketIntoConcrete for Box<dyn RenderFeatureViewSubmitPacket> {
    fn into_concrete<T: RenderFeatureViewSubmitPacket>(self) -> Box<T> {
        self.into_any().downcast::<T>().unwrap_or_else(|_| {
            panic!(
                "Unable to downcast {} into {}",
                std::any::type_name::<dyn RenderFeatureViewSubmitPacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}

/// Provides `as_concrete` method to downcast as a concrete type.
pub trait RenderFeatureViewSubmitPacketAsConcrete<'a> {
    /// Downcast `&dyn RenderFeatureViewSubmitPacket` into `&T` where `T: RenderFeatureViewSubmitPacket`.
    fn as_concrete<T: RenderFeatureViewSubmitPacket>(&'a self) -> &'a T;
}

impl<'a> RenderFeatureViewSubmitPacketAsConcrete<'a> for dyn RenderFeatureViewSubmitPacket {
    fn as_concrete<T: RenderFeatureViewSubmitPacket>(&'a self) -> &'a T {
        self.as_any().downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "Unable to downcast_ref {} into {}",
                std::any::type_name::<dyn RenderFeatureViewSubmitPacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}
