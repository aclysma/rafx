pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;
use ash::vk::Handle;

use renderer_base::PhysicalSize;
use renderer_base::LogicalSize;
use renderer_base::Window;

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
        logical_size.0 as f64 / physical_size.0 as f64
    }

    fn create_vulkan_surface(
        &self,
        _entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<vk::SurfaceKHR, vk::Result> {
        let surface_pointer = self
            .window
            .vulkan_create_surface(instance.handle().as_raw() as usize)
            .map_err(|_e| vk::Result::ERROR_INITIALIZATION_FAILED)?;
        Ok(vk::SurfaceKHR::from_raw(surface_pointer as u64))
    }

    fn extension_names(&self) -> Vec<*const i8> {
        self.window
            .vulkan_instance_extensions()
            .expect("Could not get vulkan instance extensions")
            .into_iter()
            .map(|extension| extension.as_ptr() as *const i8)
            .collect()
    }
}
