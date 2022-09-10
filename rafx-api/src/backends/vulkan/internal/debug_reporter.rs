use std::ffi::{CStr, CString};
use std::os::raw::c_void;

use ash::extensions::ext::DebugUtils;

pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use crate::vulkan::{RafxCommandBufferVulkan, RafxDeviceContextVulkan};
use crate::RafxDebugObject;

const ERRORS_TO_IGNORE: [&str; 0] = [
    // Temporary - I suspect locally built validation on M1 mac has a bug
    //"VUID-VkWriteDescriptorSet-descriptorType-00332",
    //"VUID-VkWriteDescriptorSet-descriptorType-00333",

    // windows/5700xt can return 0 max surface size when window is resized to (0, 0). Spec
    // states swapchain size must be > 0
    //"VUID-VkSwapchainCreateInfoKHR-imageExtent-01274",

    // Known issue, we allocate some static depth images at startup, this is not actually in-spec
    //"VUID-vkCmdCopyBufferToImage-commandBuffer-04477",
];

/// Callback for vulkan validation layer logging
pub extern "system" fn vulkan_debug_callback(
    flags: vk::DebugUtilsMessageSeverityFlagsEXT,
    _: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> u32 {
    if !data.is_null() {
        let data_ptr = unsafe { &(*data) };
        if !data_ptr.p_message.is_null() {
            let msg_ptr = data_ptr.p_message;
            let msg = unsafe { CStr::from_ptr(msg_ptr) };
            if flags.intersects(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
                let mut ignored = false;
                for ignored_error in &ERRORS_TO_IGNORE {
                    if msg.to_string_lossy().contains(ignored_error) {
                        ignored = true;
                        break;
                    }
                }

                if !ignored {
                    log::error!("{:?}", msg);
                    //panic!();
                }
            } else if flags.intersects(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
                log::warn!("{:?}", msg);
            } else if flags.intersects(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
                log::info!("{:?}", msg);
            } else {
                log::debug!("{:?}", msg);
            }
        } else {
            log::error!("Received null message pointer in vulkan_debug_callback");
        }
    } else {
        log::error!("Received null data pointer in vulkan_debug_callback");
    }

    vk::FALSE
}

/// Handles dropping vulkan debug reporting
pub struct VkDebugReporter {
    pub debug_report_loader: DebugUtils,
    pub debug_callback: vk::DebugUtilsMessengerEXT,
}

impl VkDebugReporter {
    /// Sets a name for an object. This is useful when debugging with a graphics debugger such as renderdoc
    /// or nsight graphics.
    pub fn set_object_name(
        &self,
        device: &RafxDeviceContextVulkan,
        object: RafxDebugObject,
        name: impl AsRef<str>,
    ) {
        let cstring = CString::new(name.as_ref()).expect("Nul in object name");

        let name_info = vk::DebugUtilsObjectNameInfoEXT::builder()
            .object_type(object.into())
            .object_handle(object.into())
            .object_name(&cstring);

        unsafe {
            // failure to set the name is not fatal/considered an error since it otherwise has no impact on the program.
            let _ = self
                .debug_report_loader
                .debug_utils_set_object_name(device.device().handle(), &name_info);
        }
    }

    /// Begins a named debug region inside a command buffer.
    pub fn cmd_begin_debug_label<T: AsRef<str>>(
        &self,
        command_buffer: &RafxCommandBufferVulkan,
        name: T,
    ) {
        let cstring = CString::new(name.as_ref()).expect("Nul in command buffer label");

        let label_info = vk::DebugUtilsLabelEXT::builder().label_name(&cstring);

        unsafe {
            let _ = self
                .debug_report_loader
                .cmd_begin_debug_utils_label(command_buffer.vk_command_buffer(), &label_info);
        }
    }

    /// Ends a previous named debug region inside a command buffer.
    pub fn cmd_end_debug_label(
        &self,
        command_buffer: &RafxCommandBufferVulkan,
    ) {
        unsafe {
            let _ = self
                .debug_report_loader
                .cmd_end_debug_utils_label(command_buffer.vk_command_buffer());
        }
    }
}

impl Drop for VkDebugReporter {
    fn drop(&mut self) {
        unsafe {
            log::trace!("destroying VkDebugReporter");
            self.debug_report_loader
                .destroy_debug_utils_messenger(self.debug_callback, None);
            log::trace!("destroyed VkDebugReporter");
        }
    }
}
