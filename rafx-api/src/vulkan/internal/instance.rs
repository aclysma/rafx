use std::ffi::{CStr, CString};

use ash::prelude::VkResult;
pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

//use super::VkEntry;
use crate::vulkan::VkCreateInstanceError::VkError;
use crate::vulkan::{VkDebugReporter, VkEntry};
use ash::extensions::ext::DebugReport;
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

/// Create one of these at startup. It never gets lost/destroyed.
pub struct VkInstance {
    pub entry: Arc<VkEntry>,
    pub instance: ash::Instance,
    pub debug_reporter: Option<VkDebugReporter>,
}

#[derive(Debug)]
pub enum VkCreateInstanceError {
    InstanceError(ash::InstanceError),
    VkError(vk::Result),
}

impl std::error::Error for VkCreateInstanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            VkCreateInstanceError::InstanceError(ref e) => Some(e),
            VkCreateInstanceError::VkError(ref e) => Some(e),
        }
    }
}

impl core::fmt::Display for VkCreateInstanceError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            VkCreateInstanceError::InstanceError(ref e) => e.fmt(fmt),
            VkCreateInstanceError::VkError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<ash::InstanceError> for VkCreateInstanceError {
    fn from(result: ash::InstanceError) -> Self {
        VkCreateInstanceError::InstanceError(result)
    }
}

impl From<vk::Result> for VkCreateInstanceError {
    fn from(result: vk::Result) -> Self {
        VkCreateInstanceError::VkError(result)
    }
}

impl VkInstance {
    /// Creates a vulkan instance.
    pub fn new(
        entry: VkEntry,
        window: &dyn HasRawWindowHandle,
        app_name: &CString,
        require_validation_layers_present: bool,
        validation_layer_debug_report_flags: vk::DebugReportFlagsEXT,
    ) -> Result<VkInstance, VkCreateInstanceError> {
        // Determine the supported version of vulkan that's available
        let vulkan_version = match entry.try_enumerate_instance_version()? {
            // Vulkan 1.1+
            Some(version) => version,
            // Vulkan 1.0
            None => vk::make_version(1, 0, 0),
        };

        let vulkan_version_tuple = (
            vk::version_major(vulkan_version),
            vk::version_minor(vulkan_version),
            vk::version_patch(vulkan_version),
        );

        log::info!("Found Vulkan version: {:?}", vulkan_version_tuple);

        // Only need 1.1 for negative y viewport support, which is also possible to get out of an
        // extension, but at this point I think 1.1 is a reasonable minimum expectation
        let minimum_version = vk::make_version(1, 1, 0);
        if vulkan_version < minimum_version {
            return Err(VkError(vk::Result::ERROR_INCOMPATIBLE_DRIVER));
        }

        // Get the available layers/extensions
        let layers = entry.enumerate_instance_layer_properties()?;
        log::debug!("Available Layers: {:#?}", layers);
        let extensions = entry.enumerate_instance_extension_properties()?;
        log::debug!("Available Extensions: {:#?}", extensions);

        // Expected to be 1.1.0 or 1.0.0 depeneding on what we found in try_enumerate_instance_version
        // https://vulkan.lunarg.com/doc/view/1.1.70.1/windows/tutorial/html/16-vulkan_1_1_changes.html

        // Info that's exposed to the driver. In a real shipped product, this data might be used by
        // the driver to make specific adjustments to improve performance
        // https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkApplicationInfo.html
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(vulkan_version);

        let mut layer_names = vec![];
        let mut extension_names = ash_window::enumerate_required_extensions(window)?;
        if !validation_layer_debug_report_flags.is_empty() {
            // Find the best validation layer that's available
            let best_validation_layer = VkInstance::find_best_validation_layer(&layers);
            if best_validation_layer.is_none() {
                if require_validation_layers_present {
                    log::error!("Could not find an appropriate validation layer. Check that the vulkan SDK has been installed or disable validation.");
                    return Err(vk::Result::ERROR_LAYER_NOT_PRESENT.into());
                } else {
                    log::warn!("Could not find an appropriate validation layer. Check that the vulkan SDK has been installed or disable validation.");
                }
            }

            let debug_extension = DebugReport::name();
            let has_debug_extension = extensions.iter().any(|extension| unsafe {
                debug_extension == CStr::from_ptr(extension.extension_name.as_ptr())
            });

            if !has_debug_extension {
                if require_validation_layers_present {
                    log::error!("Could not find the debug extension. Check that the vulkan SDK has been installed or disable validation.");
                    return Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT.into());
                } else {
                    log::warn!("Could not find the debug extension. Check that the vulkan SDK has been installed or disable validation.");
                }
            }

            if let Some(best_validation_layer) = best_validation_layer {
                if has_debug_extension {
                    layer_names.push(best_validation_layer);
                    extension_names.push(DebugReport::name());
                }
            }
        }

        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Using layers: {:?}", layer_names);
            log::debug!("Using extensions: {:?}", extension_names);
        }

        let layer_names: Vec<_> = layer_names.iter().map(|x| x.as_ptr()).collect();
        let extension_names: Vec<_> = extension_names.iter().map(|x| x.as_ptr()).collect();

        // Create the instance
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_names)
            .enabled_extension_names(&extension_names);

        log::info!("Creating vulkan instance");
        let instance: ash::Instance = unsafe { entry.create_instance(&create_info, None)? };

        // Setup the debug callback for the validation layer
        let debug_reporter = if !validation_layer_debug_report_flags.is_empty() {
            Some(Self::setup_vulkan_debug_callback(
                &entry,
                &instance,
                validation_layer_debug_report_flags,
            )?)
        } else {
            None
        };

        Ok(VkInstance {
            entry: Arc::new(entry),
            instance,
            debug_reporter,
        })
    }

    fn find_best_validation_layer(layers: &[ash::vk::LayerProperties]) -> Option<&'static CStr> {
        fn khronos_validation_layer_name() -> &'static CStr {
            CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0")
                .expect("Wrong extension string")
        }

        fn lunarg_validation_layer_name() -> &'static CStr {
            CStr::from_bytes_with_nul(b"VK_LAYER_LUNARG_standard_validation\0")
                .expect("Wrong extension string")
        }

        let khronos_validation_layer_name = khronos_validation_layer_name();
        let lunarg_validation_layer_name = lunarg_validation_layer_name();

        // Find the best validation layer that's available
        let mut best_available_layer = None;
        for layer in layers {
            let layer_name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };

            if layer_name == khronos_validation_layer_name {
                best_available_layer = Some(khronos_validation_layer_name);
                break;
            }

            if layer_name == lunarg_validation_layer_name {
                best_available_layer = Some(lunarg_validation_layer_name);
            }
        }

        best_available_layer
    }

    /// This is used to setup a debug callback for logging validation errors
    fn setup_vulkan_debug_callback<E: EntryV1_0, I: InstanceV1_0>(
        entry: &E,
        instance: &I,
        debug_report_flags: vk::DebugReportFlagsEXT,
    ) -> VkResult<VkDebugReporter> {
        log::info!("Seting up vulkan debug callback");
        let debug_info = vk::DebugReportCallbackCreateInfoEXT::builder()
            .flags(debug_report_flags)
            .pfn_callback(Some(super::debug_reporter::vulkan_debug_callback));

        let debug_report_loader = ash::extensions::ext::DebugReport::new(entry, instance);
        let debug_callback =
            unsafe { debug_report_loader.create_debug_report_callback(&debug_info, None)? };

        Ok(VkDebugReporter {
            debug_report_loader,
            debug_callback,
        })
    }
}

impl Drop for VkInstance {
    fn drop(&mut self) {
        log::trace!("destroying VkInstance");
        std::mem::drop(self.debug_reporter.take());

        unsafe {
            self.instance.destroy_instance(None);
        }

        log::trace!("destroyed VkInstance");
    }
}
