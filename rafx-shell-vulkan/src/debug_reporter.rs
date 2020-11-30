use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

use ash::extensions::ext::DebugReport;

pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

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
        error!("{:?}", msg);
    } else if flags.intersects(vk::DebugReportFlagsEXT::WARNING) {
        warn!("{:?}", msg);
    } else if flags.intersects(vk::DebugReportFlagsEXT::PERFORMANCE_WARNING) {
        warn!("{:?}", msg);
    } else if flags.intersects(vk::DebugReportFlagsEXT::INFORMATION) {
        info!("{:?}", msg);
    } else {
        debug!("{:?}", msg);
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
            trace!("destroying VkDebugReporter");
            self.debug_report_loader
                .destroy_debug_report_callback(self.debug_callback, None);
            trace!("destroyed VkDebugReporter");
        }
    }
}
