#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxApiDx12;
#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-dx12",
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2",
        feature = "rafx-gles3"
    ))
))]
use crate::empty::RafxApiEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxApiGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxApiGles3;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxApiMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxApiVulkan;

use crate::*;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

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
    #[cfg(feature = "rafx-dx12")]
    Dx12(RafxApiDx12),
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxApiVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxApiMetal),
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxApiGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxApiGles3),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
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
        _display: &dyn HasRawDisplayHandle,
        _window: &dyn HasRawWindowHandle,
        _api_def: &RafxApiDef,
    ) -> RafxResult<Self> {
        #[cfg(feature = "rafx-metal")]
        {
            return RafxApi::new_metal(_window, _api_def);
        }

        #[cfg(feature = "rafx-dx12")]
        {
            return RafxApi::new_dx12(_window, _api_def);
        }

        #[cfg(feature = "rafx-vulkan")]
        {
            return RafxApi::new_vulkan(_display, _window, _api_def);
        }

        #[cfg(feature = "rafx-gles3")]
        {
            return RafxApi::new_gles3(_display, _window, _api_def);
        }

        #[cfg(feature = "rafx-gles2")]
        {
            return RafxApi::new_gles2(_display, _window, _api_def);
        }

        return Err("Rafx was compiled with no backend feature flag. Use on of the following features: rafx-metal, rafx-vulkan, rafx-gles2")?;
    }

    /// Initialize a device using dx12
    ///
    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    #[cfg(feature = "rafx-dx12")]
    pub unsafe fn new_dx12(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Dx12(RafxApiDx12::new(
            window,
            api_def,
            &api_def.dx12_options.as_ref().unwrap_or(&Default::default()),
        )?))
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
        display: &dyn HasRawDisplayHandle,
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Vk(RafxApiVulkan::new(
            display,
            window,
            api_def,
            &api_def.vk_options.as_ref().unwrap_or(&Default::default()),
        )?))
    }

    /// Initialize a device using metal
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
    ) -> RafxResult<Self> {
        Ok(RafxApi::Metal(RafxApiMetal::new(
            window,
            api_def,
            &api_def
                .metal_options
                .as_ref()
                .unwrap_or(&Default::default()),
        )?))
    }

    /// Initialize a device using OpenGL ES 2.0
    ///
    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    #[cfg(feature = "rafx-gles2")]
    pub unsafe fn new_gles2(
        display: &dyn HasRawDisplayHandle,
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Gles2(RafxApiGles2::new(
            display,
            window,
            api_def,
            &api_def
                .gles2_options
                .as_ref()
                .unwrap_or(&Default::default()),
        )?))
    }

    /// Initialize a device using OpenGL ES 3.0
    ///
    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    #[cfg(feature = "rafx-gles3")]
    pub unsafe fn new_gles3(
        display: &dyn HasRawDisplayHandle,
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Gles3(RafxApiGles3::new(
            display,
            window,
            api_def,
            &api_def
                .gles3_options
                .as_ref()
                .unwrap_or(&Default::default()),
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
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(inner) => RafxDeviceContext::Dx12(inner.device_context().clone()),
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(inner) => RafxDeviceContext::Vk(inner.device_context().clone()),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(inner) => RafxDeviceContext::Metal(inner.device_context().clone()),
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(inner) => RafxDeviceContext::Gles2(inner.device_context().clone()),
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(inner) => RafxDeviceContext::Gles3(inner.device_context().clone()),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
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
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(inner) => inner.destroy(),
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(inner) => inner.destroy(),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(inner) => inner.destroy(),
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(inner) => inner.destroy(),
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(inner) => inner.destroy(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxApi::Empty(inner) => inner.destroy(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_api(&self) -> Option<&RafxApiDx12> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(inner) => Some(inner),
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxApi::Empty(_) => None,
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_api(&self) -> Option<&RafxApiVulkan> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxApi::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_api(&self) -> Option<&RafxApiMetal> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxApi::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_api(&self) -> Option<&RafxApiGles2> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxApi::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_api(&self) -> Option<&RafxApiGles3> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxApi::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    pub fn empty_api(&self) -> Option<&RafxApiEmpty> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxApi::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxApi::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxApi::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxApi::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxApi::Empty(inner) => Some(inner),
        }
    }
}
