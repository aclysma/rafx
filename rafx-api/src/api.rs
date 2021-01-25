#[cfg(feature = "rafx-metal")]
use crate::metal::RafxApiDefMetal;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxApiMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::{RafxApiDefVulkan, RafxApiVulkan};
use crate::*;
use raw_window_handle::HasRawWindowHandle;

/// Primary entry point to using the API. Use the `new_*` functions to initialize the desired
/// backend.
///
/// **This API object must persist for the lifetime of all objects created through it.** This
/// is verified at runtime when the API object is destroyed - either explicitly via `destroy()` or
/// by dropping the object.
///
/// Once the API object is created, use `device_context()` to obtain a cloneable handle to the
/// device. The `RafxDeviceContext` is the primary way of interacting with the API once it has been
/// initialized. These contexts and all other objects created through them must be dropped before
/// dropping `RafxApi` or calling `RafxApi::destroy()`.
pub enum RafxApi {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxApiVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxApiMetal),
}

impl RafxApi {
    /// Initialize a device using vulkan
    #[cfg(feature = "rafx-vulkan")]
    pub fn new_vulkan(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
        vk_api_def: &RafxApiDefVulkan,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Vk(RafxApiVulkan::new(
            window, api_def, vk_api_def,
        )?))
    }

    /// Initialize a device using vulkan
    #[cfg(feature = "rafx-metal")]
    pub fn new_metal(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
        vk_api_def: &RafxApiDefMetal,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Metal(RafxApiMetal::new(
            window, api_def, vk_api_def,
        )?))
    }

    /// Create a cloneable handle to the device. Most of the interaction with the graphics backend
    /// is done through this handle.
    ///
    /// The `RafxDeviceContext` does not need to be kept in scope. As long as the `RafxApi` remains
    /// in scope, dropping the device context does not do anything, and it can be obtained again
    /// by calling this function.
    ///
    /// This context is intended to be safely shared across threads. This function is thread-safe,
    /// and generally all APIs on the device context itself are thread-safe.
    pub fn device_context(&self) -> RafxDeviceContext {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(inner) => RafxDeviceContext::Vk(inner.device_context().clone()),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(inner) => RafxDeviceContext::Metal(inner.device_context().clone()),
        }
    }

    /// Destroys the graphics API instance. Any `RafxDeviceContext` created through this API, and
    /// any object created through those device contexts, must be dropped before calling destroy()
    ///
    /// `destroy()` is automatically called if RafxApi is dropped and it has not yet been called, so
    /// it is not necessary to call this function explicitly.
    pub fn destroy(&mut self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(inner) => inner.destroy(),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(inner) => inner.destroy(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_api(&self) -> Option<&RafxApiVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_api(&self) -> Option<&RafxApiMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(inner) => Some(inner),
        }
    }
}
