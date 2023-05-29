extern crate ash;

// Represents a dynamic or static-linked entry to the vulkan API
pub struct VkEntry(ash::Entry);

impl VkEntry {
    #[cfg(feature = "static-vulkan")]
    pub fn new_static() -> Result<Self, ash::LoadingError> {
        let static_fn = ash::vk::StaticFn {
            get_instance_proc_addr,
        };

        let entry = unsafe { ash::Entry::from_static_fn(static_fn) };

        Ok(VkEntry(entry))
    }

    pub fn new_dynamic() -> Result<Self, ash::LoadingError> {
        unsafe {
            let entry = ash::Entry::load()?;
            Ok(VkEntry(entry))
        }
    }

    pub fn entry(&self) -> &ash::Entry {
        &self.0
    }
}

#[cfg(feature = "static-vulkan")]
extern "system" {
    fn vkGetInstanceProcAddr(
        instance: vk::Instance,
        p_name: *const std::os::raw::c_char,
    ) -> vk::PFN_vkVoidFunction;
}

#[cfg(feature = "static-vulkan")]
extern "system" fn get_instance_proc_addr(
    instance: vk::Instance,
    p_name: *const std::os::raw::c_char,
) -> vk::PFN_vkVoidFunction {
    unsafe { vkGetInstanceProcAddr(instance, p_name) }
}
