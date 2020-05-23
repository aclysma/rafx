use std::ffi::CString;

pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;
use ash::prelude::VkResult;

use super::Window;
use super::debug_reporter;
use super::VkDebugReporter;
use ash::extensions::ext::DebugReport;

/// Create one of these at startup. It never gets lost/destroyed.
pub struct VkInstance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub debug_reporter: Option<VkDebugReporter>,
}

#[derive(Debug)]
pub enum VkCreateInstanceError {
    LoadingError(ash::LoadingError),
    InstanceError(ash::InstanceError),
    VkError(vk::Result),
}

impl std::error::Error for VkCreateInstanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            VkCreateInstanceError::LoadingError(ref e) => Some(e),
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
            VkCreateInstanceError::LoadingError(ref e) => e.fmt(fmt),
            VkCreateInstanceError::InstanceError(ref e) => e.fmt(fmt),
            VkCreateInstanceError::VkError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<ash::LoadingError> for VkCreateInstanceError {
    fn from(result: ash::LoadingError) -> Self {
        VkCreateInstanceError::LoadingError(result)
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
        window: &dyn Window,
        app_name: &CString,
        validation_layer_debug_report_flags: vk::DebugReportFlagsEXT,
    ) -> Result<VkInstance, VkCreateInstanceError> {
        // This loads the dll/so if needed
        info!("Finding vulkan entry point");
        let entry = ash::Entry::new()?;

        // Determine the supported version of vulkan that's available
        let vulkan_version = match entry.try_enumerate_instance_version()? {
            // Vulkan 1.1+
            Some(version) => {
                let major = vk::version_major(version);
                let minor = vk::version_minor(version);
                let patch = vk::version_patch(version);

                (major, minor, patch)
            }
            // Vulkan 1.0
            None => (1, 0, 0),
        };

        info!("Found Vulkan version: {:?}", vulkan_version);

        // Get the available layers/extensions
        let layers = entry.enumerate_instance_layer_properties()?;
        debug!("Available Layers: {:#?}", layers);
        let extensions = entry.enumerate_instance_extension_properties()?;
        debug!("Available Extensions: {:#?}", extensions);

        // Expected to be 1.1.0 or 1.0.0 depeneding on what we found in try_enumerate_instance_version
        // https://vulkan.lunarg.com/doc/view/1.1.70.1/windows/tutorial/html/16-vulkan_1_1_changes.html
        let api_version = vk::make_version(vulkan_version.0, vulkan_version.1, 0);

        // Info that's exposed to the driver. In a real shipped product, this data might be used by
        // the driver to make specific adjustments to improve performance
        // https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/VkApplicationInfo.html
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(app_name)
            .application_version(0)
            .engine_name(app_name)
            .engine_version(0)
            .api_version(api_version);

        // Determine what layers to use
        let validation_layer_name = CString::new("VK_LAYER_LUNARG_standard_validation").unwrap();

        let mut layer_names = vec![];
        if !validation_layer_debug_report_flags.is_empty() {
            //TODO: Validate that the layer exists
            //if layers.iter().find(|x| CStr::from_bytes_with_nul(&x.layer_name) == &validation_layer_name) {
            layer_names.push(validation_layer_name);
            //}
        }

        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        // Determine what extensions to use
        let mut extension_names_raw = window.extension_names();

        if !validation_layer_debug_report_flags.is_empty() {
            extension_names_raw.push(DebugReport::name().as_ptr())
        }

        // Create the instance
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        info!("Creating vulkan instance");
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
            entry,
            instance,
            debug_reporter,
        })
    }

    /// This is used to setup a debug callback for logging validation errors
    fn setup_vulkan_debug_callback(
        entry: &ash::Entry,
        instance: &ash::Instance,
        debug_report_flags: vk::DebugReportFlagsEXT,
    ) -> VkResult<VkDebugReporter> {
        info!("Seting up vulkan debug callback");
        let debug_info = vk::DebugReportCallbackCreateInfoEXT::builder()
            .flags(debug_report_flags)
            .pfn_callback(Some(debug_reporter::vulkan_debug_callback));

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
        trace!("destroying VkInstance");
        std::mem::drop(self.debug_reporter.take());

        unsafe {
            self.instance.destroy_instance(None);
        }

        trace!("destroyed VkInstance");
    }
}
