use std::ffi::{CStr, CString};

use ash::prelude::VkResult;
use ash::vk;

//use super::VkEntry;
use crate::vulkan::{VkDebugReporter, VkEntry};
use crate::{RafxError, RafxResult};
use ash::extensions::ext::DebugUtils;
use ash::vk::DebugUtilsMessageTypeFlagsEXT;
use raw_window_handle::HasRawDisplayHandle;
use std::sync::Arc;

/// Create one of these at startup. It never gets lost/destroyed.
pub struct VkInstance {
    pub entry: Arc<VkEntry>,
    pub instance: ash::Instance,
    pub debug_reporter: Option<Arc<VkDebugReporter>>,
}

impl VkInstance {
    /// Creates a vulkan instance.
    pub fn new(
        entry: VkEntry,
        display: &dyn HasRawDisplayHandle,
        app_name: &CString,
        require_validation_layers_present: bool,
        validation_layer_debug_report_flags: vk::DebugUtilsMessageSeverityFlagsEXT,
        enable_debug_names: bool,
    ) -> RafxResult<VkInstance> {
        // Determine the supported version of vulkan that's available
        let vulkan_version = match entry.entry().try_enumerate_instance_version()? {
            // Vulkan 1.1+
            Some(version) => version,
            // Vulkan 1.0
            None => vk::make_api_version(0, 1, 0, 0),
        };

        let vulkan_version_tuple = (
            vk::api_version_major(vulkan_version),
            vk::api_version_minor(vulkan_version),
            vk::api_version_patch(vulkan_version),
        );

        log::info!("Found Vulkan version: {:?}", vulkan_version_tuple);

        // Only need 1.1 for negative y viewport support, which is also possible to get out of an
        // extension, but at this point I think 1.1 is a reasonable minimum expectation
        let minimum_version = vk::make_api_version(0, 1, 1, 0);
        if vulkan_version < minimum_version {
            return Err(RafxError::VkError(vk::Result::ERROR_INCOMPATIBLE_DRIVER))?;
        }

        // Get the available layers/extensions
        let layers = entry.entry().enumerate_instance_layer_properties()?;
        log::debug!("Available Layers: {:#?}", layers);
        let extensions = entry
            .entry()
            .enumerate_instance_extension_properties(None)?;
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
        let mut extension_names: Vec<_> =
            ash_window::enumerate_required_extensions(display.raw_display_handle())?
                .iter()
                .copied()
                .collect();

        let debug_utils_extension_available = extensions.iter().any(|extension|
            unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) } == DebugUtils::name()
        );

        let mut use_debug_utils_extension = debug_utils_extension_available && enable_debug_names;
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

            if !debug_utils_extension_available {
                if require_validation_layers_present {
                    log::error!("Could not find the DebugUtils extension. Check that the vulkan SDK has been installed or disable validation.");
                    return Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT.into());
                } else {
                    log::warn!("Could not find the DebugUtils extension. Check that the vulkan SDK has been installed or disable validation.");
                }
            }

            if let Some(best_validation_layer) = best_validation_layer {
                if debug_utils_extension_available {
                    layer_names.push(best_validation_layer);
                    use_debug_utils_extension |= true;
                }
            }
        }

        if use_debug_utils_extension {
            extension_names.push(DebugUtils::name().as_ptr());
        }

        if extensions.iter().any(|extension|
             unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) } == vk::ExtSwapchainColorspaceFn::name()
        ) {
            extension_names.push(vk::ExtSwapchainColorspaceFn::name().as_ptr());
        }

        #[allow(unused_mut)]
        let mut create_instance_flags = vk::InstanceCreateFlags::empty();

        // Required to support MoltenVK 1.3
        #[cfg(target_os = "macos")]
        {
            // From https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkInstanceCreateFlagBits.html
            const VK_INSTANCE_CREATE_ENUMERATE_PORTABILITY_BIT_KHR: vk::InstanceCreateFlags =
                vk::InstanceCreateFlags::from_raw(0x00000001);

            fn khr_portability_subset_extension_name() -> &'static CStr {
                CStr::from_bytes_with_nul(b"VK_KHR_portability_enumeration\0")
                    .expect("Wrong extension string")
            }

            let swapchain_extension_name = khr_portability_subset_extension_name();
            if extensions.iter().any(|extension| unsafe {
                CStr::from_ptr(extension.extension_name.as_ptr()) == swapchain_extension_name
            }) {
                extension_names.push(swapchain_extension_name);
                create_instance_flags |= VK_INSTANCE_CREATE_ENUMERATE_PORTABILITY_BIT_KHR;
            }
        }

        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Using layers: {:?}", layer_names);
            log::debug!("Using extensions: {:?}", extension_names);
        }

        let layer_names: Vec<_> = layer_names.iter().map(|x| x.as_ptr()).collect();
        //let extension_names: Vec<_> = extension_names.iter().map(|&x| x).collect();

        // Create the instance
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_names)
            .enabled_extension_names(&extension_names)
            .flags(create_instance_flags);

        log::info!("Creating vulkan instance");
        let instance: ash::Instance = unsafe { entry.entry().create_instance(&create_info, None)? };

        // Setup the debug callback for the validation layer
        let debug_reporter = if use_debug_utils_extension {
            Some(Self::setup_vulkan_debug_reporter(
                &entry.entry(),
                &instance,
                validation_layer_debug_report_flags,
            )?)
        } else {
            None
        };

        Ok(VkInstance {
            entry: Arc::new(entry),
            instance,
            debug_reporter: debug_reporter.map(Arc::new),
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
    fn setup_vulkan_debug_reporter(
        entry: &ash::Entry,
        instance: &ash::Instance,
        debug_report_flags: vk::DebugUtilsMessageSeverityFlagsEXT,
    ) -> VkResult<VkDebugReporter> {
        log::info!("Seting up vulkan debug callback");
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(debug_report_flags)
            .message_type(
                DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(super::debug_reporter::vulkan_debug_callback));

        let debug_report_loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let debug_callback =
            unsafe { debug_report_loader.create_debug_utils_messenger(&debug_info, None)? };

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
