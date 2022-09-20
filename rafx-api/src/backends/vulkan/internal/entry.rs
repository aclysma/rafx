extern crate ash;

#[cfg(feature = "static-vulkan")]
extern "system" {
    fn vkGetInstanceProcAddr(
        instance: vk::Instance,
        p_name: *const std::os::raw::c_char,
    ) -> vk::PFN_vkVoidFunction;
}

/// Fetches the function pointer to `vkGetInstanceProcAddr` which is statically linked.
#[cfg(feature = "static-vulkan")]
pub fn load_static() -> ash::Entry {
    let static_fn = vk::StaticFn {
        get_instance_proc_addr: vkGetInstanceProcAddr,
    };
    unsafe { ash::Entry::from_static_fn(static_fn) }
}

pub enum VkEntry {
    Dynamic(ash::Entry),
    #[cfg(feature = "static-vulkan")]
    Static(ash::Entry),
}

impl VkEntry {
    #[cfg(feature = "static-vulkan")]
    pub fn new_static() -> Result<Self, ash::LoadingError> {
        let entry = load_static()?;
        Ok(VkEntry::Static(entry))
    }

    pub fn new_dynamic() -> Result<Self, ash::LoadingError> {
        unsafe {
            let entry = ash::Entry::load()?;
            Ok(VkEntry::Dynamic(entry))
        }
    }

    pub fn entry(&self) -> &ash::Entry {
        match &self {
            VkEntry::Dynamic(entry) => entry,
            #[cfg(feature = "static-vulkan")]
            VkEntry::Static(entry) => entry,
        }
    }
}
