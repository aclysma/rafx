#[cfg(feature = "rafx-metal")]
use crate::metal::RafxApiMetal;
use crate::vulkan::{RafxApiDefVulkan, RafxApiVulkan};
use crate::*;
use raw_window_handle::HasRawWindowHandle;

/// Create a device using the given API. Generally processes only need one device.
pub enum RafxApi {
    Vk(RafxApiVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxApiMetal),
}

impl RafxApi {
    /// Initialize a device using vulkan
    pub fn new_vulkan(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
        vk_api_def: &RafxApiDefVulkan,
    ) -> RafxResult<Self> {
        Ok(RafxApi::Vk(RafxApiVulkan::new(
            window, api_def, vk_api_def,
        )?))
    }

    pub fn device_context(&self) -> RafxDeviceContext {
        match self {
            RafxApi::Vk(inner) => RafxDeviceContext::Vk(inner.device_context().clone()),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn destroy(&mut self) -> RafxResult<()> {
        match self {
            RafxApi::Vk(inner) => inner.destroy(),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_device(&self) -> Option<&RafxApiVulkan> {
        match self {
            RafxApi::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxApi::Metal(_) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_device(&self) -> Option<&RafxApiMetal> {
        match self {
            RafxApi::Vk(_) => None,
            RafxApi::Metal(inner) => Some(inner),
        }
    }
}
