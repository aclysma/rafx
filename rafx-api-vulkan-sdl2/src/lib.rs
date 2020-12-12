pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use ash::prelude::VkResult;
use rafx_api_vulkan::LogicalSize;
use rafx_api_vulkan::PhysicalSize;
use rafx_api_vulkan::VkEntry;
use rafx_api_vulkan::Window;
use std::ffi::CStr;

pub struct Sdl2Window<'a> {
    window: &'a sdl2::video::Window,
}

impl<'a> Sdl2Window<'a> {
    pub fn new(window: &'a sdl2::video::Window) -> Self {
        Sdl2Window { window }
    }
}

impl<'a> Window for Sdl2Window<'a> {
    fn physical_size(&self) -> PhysicalSize {
        let physical_size = self.window.vulkan_drawable_size();
        PhysicalSize::new(physical_size.0, physical_size.1)
    }

    fn logical_size(&self) -> LogicalSize {
        let logical_size = self.window.size();
        LogicalSize::new(logical_size.0, logical_size.1)
    }

    fn scale_factor(&self) -> f64 {
        let physical_size = self.window.vulkan_drawable_size();
        let logical_size = self.window.size();
        physical_size.0 as f64 / logical_size.0 as f64
    }

    unsafe fn create_vulkan_surface(
        &self,
        entry: &VkEntry,
        instance: &ash::Instance,
    ) -> VkResult<vk::SurfaceKHR> {
        ash_window::create_surface(entry, instance, self.window, None)
    }

    fn extension_names(&self) -> VkResult<Vec<&'static CStr>> {
        ash_window::enumerate_required_extensions(self.window)
    }
}
