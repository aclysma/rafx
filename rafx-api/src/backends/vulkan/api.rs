use super::internal::*;
use ash::vk;
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

use crate::vulkan::{RafxDeviceContextVulkan, RafxDeviceContextVulkanInner};
use crate::*;
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
        #[cfg(not(feature = "static-vulkan"))]
        let link_method = VulkanLinkMethod::Dynamic;
        #[cfg(feature = "static-vulkan")]
        let link_method = VulkanLinkMethod::Static;

        link_method
    }
}

/// Vulkan-specific configuration
pub struct RafxApiDefVulkan {
    /// Used as a hint for drivers for what is being run. There are no special requirements for
    /// this. It is not visible to end-users.
    pub app_name: CString,

    /// Defines whether to load vulkan dynamically or use a statically-linked implementation. A
    /// common case where static linking is useful is linking MoltenVK on iOS devices
    pub link_method: VulkanLinkMethod,

    /// Used to enable/disable validation at runtime. Not all APIs allow this. Validation is helpful
    /// during development but very expensive. Applications should not ship with validation enabled.
    pub validation_mode: RafxValidationMode,

    /// Override the default enabled features with a custom set of features
    pub physical_device_features: Option<vk::PhysicalDeviceFeatures>,
    // The OS-specific layers/extensions are already included. Debug layers/extension are included
    // if enable_validation is true
    //TODO: Additional instance layer names
    //TODO: Additional instance extension names
    //TODO: Additional device extension names
}

impl Default for RafxApiDefVulkan {
    fn default() -> Self {
        RafxApiDefVulkan {
            app_name: CString::new("Rafx Application").unwrap(),
            link_method: Default::default(),
            validation_mode: Default::default(),
            physical_device_features: None,
        }
    }
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
    pub fn device_context(&self) -> &RafxDeviceContextVulkan {
        self.device_context.as_ref().unwrap()
    }

    pub fn vk_instance(&self) -> &ash::Instance {
        &self.instance.instance
    }

    /// # Safety
    ///
    /// GPU programming is fundamentally unsafe, so all rafx APIs that interact with the GPU should
    /// be considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    /// behavior on the CPU for reasons other than interacting with the GPU.
    pub unsafe fn new(
        window: &dyn HasRawWindowHandle,
        _api_def: &RafxApiDef,
        vk_api_def: &RafxApiDefVulkan,
    ) -> RafxResult<Self> {
        let link_method = vk_api_def.link_method;
        let app_name = vk_api_def.app_name.clone();

        let (require_validation_layers_present, validation_layer_debug_report_flags) =
            match vk_api_def.validation_mode {
                RafxValidationMode::Disabled => {
                    (false, vk::DebugUtilsMessageSeverityFlagsEXT::empty())
                }
                RafxValidationMode::EnabledIfAvailable => {
                    (false, vk::DebugUtilsMessageSeverityFlagsEXT::all())
                }
                RafxValidationMode::Enabled => (true, vk::DebugUtilsMessageSeverityFlagsEXT::all()),
            };

        log::info!("Validation mode: {:?}", vk_api_def.validation_mode);
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

        let inner = Arc::new(RafxDeviceContextVulkanInner::new(
            &instance,
            &vk_api_def.physical_device_features,
        )?);
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
                Ok(inner) => std::mem::drop(inner),
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
