use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

use ash::extensions::ext::DebugReport;

pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

const ERRORS_TO_IGNORE: [&'static str; 0] = [
    // Temporary - I suspect locally built validation on M1 mac has a bug
    //"VUID-VkWriteDescriptorSet-descriptorType-00332",
    //"VUID-VkWriteDescriptorSet-descriptorType-00333",
    // windows/5700xt can return 0 max surface size when window is resized to (0, 0). Spec
    // states swapchain size must be > 0
    //"VUID-VkSwapchainCreateInfoKHR-imageExtent-01274",
];

/// Callback for vulkan validation layer logging
pub extern "system" fn vulkan_debug_callback(
    flags: vk::DebugReportFlagsEXT,
    _: vk::DebugReportObjectTypeEXT,
    _: u64,
    _: usize,
    _: i32,
    _: *const c_char,
    p_message: *const c_char,
    _: *mut c_void,
) -> u32 {
    let msg = unsafe { CStr::from_ptr(p_message) };
    if flags.intersects(vk::DebugReportFlagsEXT::ERROR) {
        let mut ignored = false;
        for ignored_error in &ERRORS_TO_IGNORE {
            if msg.to_string_lossy().contains(ignored_error) {
                ignored = true;
                break;
            }
        }

        if !ignored {
            log::error!("{:?}", msg);
            panic!();
        }
    } else if flags.intersects(vk::DebugReportFlagsEXT::WARNING) {
        log::warn!("{:?}", msg);
    } else if flags.intersects(vk::DebugReportFlagsEXT::PERFORMANCE_WARNING) {
        log::warn!("{:?}", msg);
    } else if flags.intersects(vk::DebugReportFlagsEXT::INFORMATION) {
        log::info!("{:?}", msg);
    } else {
        log::debug!("{:?}", msg);
    }

    vk::FALSE
}

/// Handles dropping vulkan debug reporting
pub struct VkDebugReporter {
    pub debug_report_loader: DebugReport,
    pub debug_callback: vk::DebugReportCallbackEXT,
}

impl Drop for VkDebugReporter {
    fn drop(&mut self) {
        unsafe {
            log::trace!("destroying VkDebugReporter");
            self.debug_report_loader
                .destroy_debug_report_callback(self.debug_callback, None);
            log::trace!("destroyed VkDebugReporter");
        }
    }
}
