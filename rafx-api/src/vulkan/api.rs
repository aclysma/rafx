use super::internal::*;
use crate::{RafxApiDef, RafxResult, RafxValidationMode};
use ash::vk;
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

use crate::vulkan::internal::device::VkDeviceContext;
use crate::vulkan::{RafxDeviceContextVulkan, RafxDeviceContextVulkanInner};
use std::ffi::CString;

/// Determines the method of finding the vulkan loader
#[derive(Copy, Clone, Debug)]
pub enum VulkanLinkMethod {
    /// Link vulkan dynamically (recommended and default)
    Dynamic,

    /// Assume the vulkan loader is statically linked.. intended for platforms like iOS
    #[cfg(feature = "static-vulkan")]
    Static,
}

impl Default for VulkanLinkMethod {
    fn default() -> Self {
        VulkanLinkMethod::Dynamic
    }
}

/// Vulkan-specific configuration
#[derive(Default)]
pub struct RafxApiDefVulkan {
    /// Used as a hint for drivers for what is being run. There are no special requirements for
    /// this. It is not visible to end-users.
    pub app_name: Option<CString>,

    /// Defines whether to load vulkan dynamically or use a statically-linked implementation. A
    /// common case where static linking is useful is linking MoltenVK on iOS devices
    pub link_method: Option<VulkanLinkMethod>,
    // The OS-specific layers/extensions are already included. Debug layers/extension are included
    // if enable_validation is true
    //TODO: Additional instance layer names
    //TODO: Additional instance extension names
    //TODO: Additional device extension names
}

pub struct RafxApiVulkan {
    instance: VkInstance,
    device_context: Option<RafxDeviceContextVulkan>,
}

impl Drop for RafxApiVulkan {
    fn drop(&mut self) {
        self.destroy().unwrap();
    }
}

impl RafxApiVulkan {
    //TEMPORARY
    pub fn vk_device_context(&self) -> &VkDeviceContext {
        self.device_context.as_ref().unwrap().vk_device_context()
    }

    pub fn device_context(&self) -> &RafxDeviceContextVulkan {
        self.device_context.as_ref().unwrap()
    }

    pub fn vk_instance(&self) -> &ash::Instance {
        &self.instance.instance
    }

    pub fn new(
        window: &dyn HasRawWindowHandle,
        api_def: &RafxApiDef,
        vk_api_def: &RafxApiDefVulkan,
    ) -> RafxResult<Self> {
        let link_method = vk_api_def.link_method.clone().unwrap_or_default();
        let app_name = vk_api_def
            .app_name
            .clone()
            .unwrap_or_else(|| CString::new("Rafx Application").unwrap());

        let (require_validation_layers_present, validation_layer_debug_report_flags) =
            match api_def.validation_mode {
                RafxValidationMode::Disabled => (false, vk::DebugReportFlagsEXT::empty()),
                RafxValidationMode::EnabledIfAvailable => (false, vk::DebugReportFlagsEXT::all()),
                RafxValidationMode::Enabled => (true, vk::DebugReportFlagsEXT::all()),
            };

        log::info!("Link method for vulkan: {:?}", link_method);
        let entry = match link_method {
            VulkanLinkMethod::Dynamic => VkEntry::new_dynamic(),
            #[cfg(feature = "static-vulkan")]
            VulkanLinkMethod::Static => VkEntry::new_static(),
        }?;

        let instance = VkInstance::new(
            entry,
            window,
            &app_name,
            require_validation_layers_present,
            validation_layer_debug_report_flags,
        )?;

        let inner = Arc::new(RafxDeviceContextVulkanInner::new(&instance)?);
        let device_context = RafxDeviceContextVulkan::new(inner)?;

        Ok(RafxApiVulkan {
            instance,
            device_context: Some(device_context),
        })
    }

    pub(crate) fn destroy(&mut self) -> RafxResult<()> {
        if let Some(device_context) = self.device_context.take() {
            // Clear any internal caches that may hold references to the device
            let inner = device_context.inner.clone();
            inner.descriptor_heap.clear_pools(device_context.device());
            inner.resource_cache.clear_caches();

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            let _create_index = device_context.create_index;

            // Thsi should be the final device context
            std::mem::drop(device_context);

            let _strong_count = Arc::strong_count(&inner);
            match Arc::try_unwrap(inner) {
                Ok(mut inner) => unsafe { inner.device_context.destroy() },
                Err(_arc) => {
                    Err(format!(
                        "Could not destroy device, {} references to it exist",
                        _strong_count
                    ))?;

                    #[cfg(debug_assertions)]
                    #[cfg(feature = "track-device-contexts")]
                    {
                        let mut all_contexts = _arc.all_contexts.lock().unwrap();
                        all_contexts.remove(&_create_index);
                        for (k, v) in all_contexts.iter_mut() {
                            v.resolve();
                            println!("context allocation: {}\n{:?}", k, v);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
