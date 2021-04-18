#[cfg(any(
    feature = "rafx-empty",
    not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
))]
use crate::empty::RafxApiEmpty;
#[cfg(feature = "rafx-metal")]
use crate::metal::{RafxApiDefMetal, RafxApiMetal};
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::{RafxApiDefVulkan, RafxApiVulkan};
#[cfg(feature = "rafx-gl")]
use crate::gl::{RafxApiDefGl, RafxApiGl};
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
    #[cfg(feature = "rafx-gl")]
    Gl(RafxApiGl),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
    ))]
    Empty(RafxApiEmpty),
}

impl RafxApi {
    /// Create a device using the "default" backend for the platform.
    ///
    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    #[allow(unreachable_code)]
    pub unsafe fn new(
        _window: &dyn HasRawWindowHandle,
        _api_def: &RafxApiDef,
    ) -> RafxResult<Self> {
        #[cfg(feature = "rafx-metal")]
        {
            return RafxApi::new_metal(_window, _api_def, &Default::default());
        }

        #[cfg(feature = "rafx-vulkan")]
        {
            return RafxApi::new_vulkan(_window, _api_def, &Default::default());
        }

        #[cfg(feature = "rafx-gl")]
        {
            return RafxApi::new_gl(_window, _api_def, &Default::default());
        }

        return Err("Rafx was compiled with no backend feature flag. Use on of the following features: rafx-metal, rafx-vulkan, rafx-gl")?;
    }

    /// Initialize a device using vulkan
    ///
    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    #[cfg(feature = "rafx-vulkan")]
    pub unsafe fn new_vulkan(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
        vk_api_def: &RafxApiDefVulkan,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Vk(RafxApiVulkan::new(
            window, api_def, vk_api_def,
        )?))
    }

    /// Initialize a device using vulkan
    ///
    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    #[cfg(feature = "rafx-metal")]
    pub unsafe fn new_metal(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
        vk_api_def: &RafxApiDefMetal,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Metal(RafxApiMetal::new(
            window, api_def, vk_api_def,
        )?))
    }

    /// Initialize a device using vulkan
    #[cfg(feature = "rafx-gl")]
    pub fn new_gl(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
        gl_api_def: &RafxApiDefGl,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Gl(RafxApiGl::new(
            window, api_def, gl_api_def,
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
            #[cfg(feature = "rafx-gl")]
            RafxApi::Gl(inner) => RafxDeviceContext::Gl(inner.device_context().clone()),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxApi::Empty(inner) => RafxDeviceContext::Empty(inner.device_context().clone()),
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
            #[cfg(feature = "rafx-gl")]
            RafxApi::Gl(inner) => inner.destroy(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxApi::Empty(inner) => inner.destroy(),
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
            #[cfg(feature = "rafx-gl")]
            RafxApi::Gl(inner) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxApi::Empty(_) => None,
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
            #[cfg(feature = "rafx-gl")]
            RafxApi::Gl(inner) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxApi::Empty(_) => None,
        }
    }


    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gl")]
    pub fn gl_api(&self) -> Option<&RafxApiGl> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(inner) => None,
            #[cfg(feature = "rafx-gl")]
            RafxApi::Gl(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxApi::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
    ))]
    pub fn empty_api(&self) -> Option<&RafxApiEmpty> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
            #[cfg(feature = "rafx-gl")]
            RafxApi::Gl(inner) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxApi::Empty(inner) => Some(inner),
        }
    }
}
