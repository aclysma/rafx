use crate::render_features::render_features_prelude::*;
use downcast_rs::{impl_downcast, Downcast};

/// The index of a `RenderView` in this `RenderFeature`'s `FramePacket` for the current frame.
pub type ViewFrameIndex = u32;

/// The `ID` of a `RenderObjectInstance` in a specific `ViewPacket` for the current frame.
pub type RenderObjectInstancePerViewId = u32;

impl_downcast!(RenderFeatureViewPacket);
/// A type-erased trait used by the `Renderer`, `RenderFrameJob`, and `RendererThreadPool`
/// to control the workload of the rendering process without identifying specific types
/// used in each `RenderFeature`'s frame packet or workload. See `ViewPacket` for implementation
/// details.
pub trait RenderFeatureViewPacket: Downcast + Sync + Send {
    fn view(&self) -> &RenderView;

    fn num_render_object_instances(&self) -> usize;

    fn push_render_object_instance(
        &mut self,
        render_object_instance_id: RenderObjectInstanceId,
        render_object_instance: RenderObjectInstance,
    ) -> RenderObjectInstancePerViewId;

    fn push_volume(
        &mut self,
        object_id: ObjectId,
    );
}

/// Provides `into_concrete` method to downcast into a concrete type.
pub trait RenderFeatureViewPacketIntoConcrete {
    /// Downcast `Box<dyn RenderFeatureViewPacket>` into `Box<T>` where `T: RenderFeatureViewPacket`.
    fn into_concrete<T: RenderFeatureViewPacket>(self) -> Box<T>;
}

impl RenderFeatureViewPacketIntoConcrete for Box<dyn RenderFeatureViewPacket> {
    fn into_concrete<T: RenderFeatureViewPacket>(self) -> Box<T> {
        self.into_any().downcast::<T>().unwrap_or_else(|_| {
            panic!(
                "Unable to downcast {} into {}",
                std::any::type_name::<dyn RenderFeatureViewPacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}

/// Provides `as_concrete` method to downcast as a concrete type.
pub trait RenderFeatureViewPacketAsConcrete<'a> {
    /// Downcast `&dyn RenderFeatureViewPacket` into `&T` where `T: RenderFeatureViewPacket`.
    fn as_concrete<T: RenderFeatureViewPacket>(&'a self) -> &'a T;
}

impl<'a> RenderFeatureViewPacketAsConcrete<'a> for dyn RenderFeatureViewPacket {
    fn as_concrete<T: RenderFeatureViewPacket>(&'a self) -> &'a T {
        self.as_any().downcast_ref::<T>().unwrap_or_else(|| {
            panic!(
                "Unable to downcast_ref {} into {}",
                std::any::type_name::<dyn RenderFeatureViewPacket>(),
                std::any::type_name::<T>(),
            )
        })
    }
}
