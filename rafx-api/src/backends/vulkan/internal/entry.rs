extern crate ash;
use self::ash::prelude::VkResult;
use ash::{version::EntryV1_0, vk, Instance, InstanceError};

// Represents a dynamic or static-linked entry to the vulkan API
pub enum VkEntry {
    Dynamic(ash::Entry),
    #[cfg(feature = "static-vulkan")]
    Static(MoltenEntry),
}

impl VkEntry {
    #[cfg(feature = "static-vulkan")]
    pub fn new_static() -> Result<Self, ash::LoadingError> {
        let entry = crate::entry::MoltenEntry::load()?;
        Ok(VkEntry::Static(entry))
    }

    pub fn new_dynamic() -> Result<Self, ash::LoadingError> {
        unsafe {
            let entry = ash::Entry::new()?;
            Ok(VkEntry::Dynamic(entry))
        }
    }

    pub fn try_enumerate_instance_version(&self) -> VkResult<Option<u32>> {
        match &self {
            VkEntry::Dynamic(entry) => entry.try_enumerate_instance_version(),
            #[cfg(feature = "static-vulkan")]
            VkEntry::Static(entry) => entry.try_enumerate_instance_version(),
        }
    }
}

impl EntryV1_0 for VkEntry {
    type Instance = Instance;
    #[doc = "<https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCreateInstance.html>"]
    unsafe fn create_instance(
        &self,
        create_info: &vk::InstanceCreateInfo,
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    ) -> Result<Self::Instance, InstanceError> {
        match &self {
            VkEntry::Dynamic(entry) => entry.create_instance(create_info, allocation_callbacks),
            #[cfg(feature = "static-vulkan")]
            VkEntry::Static(entry) => entry.create_instance(create_info, allocation_callbacks),
        }
    }
    fn fp_v1_0(&self) -> &vk::EntryFnV1_0 {
        match &self {
            VkEntry::Dynamic(entry) => entry.fp_v1_0(),
            #[cfg(feature = "static-vulkan")]
            VkEntry::Static(entry) => entry.fp_v1_0(),
        }
    }
    fn static_fn(&self) -> &vk::StaticFn {
        match &self {
            VkEntry::Dynamic(entry) => entry.static_fn(),
            #[cfg(feature = "static-vulkan")]
            VkEntry::Static(entry) => entry.static_fn(),
        }
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

//This code based on ash-molten. However, the crate is small and I found the most reliable way to
//get iOS builds going was to embed this directly.

/// The entry point for the statically linked molten library
#[cfg(feature = "static-vulkan")]
pub struct MoltenEntry {
    static_fn: vk::StaticFn,
    entry_fn_1_0: vk::EntryFnV1_0,
}

#[cfg(feature = "static-vulkan")]
impl MoltenEntry {
    /// Fetches the function pointer to `get_instance_proc_addr` which is statically linked. This
    /// function can not fail.
    pub fn load() -> Result<MoltenEntry, ash::LoadingError> {
        let static_fn = vk::StaticFn {
            get_instance_proc_addr,
        };

        let entry_fn_1_0 = vk::EntryFnV1_0::load(|name| unsafe {
            std::mem::transmute(
                static_fn.get_instance_proc_addr(vk::Instance::null(), name.as_ptr()),
            )
        });

        Ok(MoltenEntry {
            static_fn,
            entry_fn_1_0,
        })
    }

    // This is copied over from ash
    pub fn try_enumerate_instance_version(&self) -> VkResult<Option<u32>> {
        unsafe {
            let mut api_version = 0;
            let enumerate_instance_version: Option<vk::PFN_vkEnumerateInstanceVersion> = {
                let name = b"vkEnumerateInstanceVersion\0".as_ptr() as *const _;
                std::mem::transmute(
                    self.static_fn()
                        .get_instance_proc_addr(vk::Instance::null(), name),
                )
            };
            if let Some(enumerate_instance_version) = enumerate_instance_version {
                let err_code = (enumerate_instance_version)(&mut api_version);
                match err_code {
                    vk::Result::SUCCESS => Ok(Some(api_version)),
                    _ => Err(err_code),
                }
            } else {
                Ok(None)
            }
        }
    }
}

#[cfg(feature = "static-vulkan")]
impl EntryV1_0 for MoltenEntry {
    type Instance = Instance;
    #[doc = "<https://www.khronos.org/registry/vulkan/specs/1.1-extensions/man/html/vkCreateInstance.html>"]
    unsafe fn create_instance(
        &self,
        create_info: &vk::InstanceCreateInfo,
        allocation_callbacks: Option<&vk::AllocationCallbacks>,
    ) -> Result<Self::Instance, InstanceError> {
        use crate::entry::ash::RawPtr;

        let mut instance: vk::Instance = vk::Instance::null();
        let err_code = self.fp_v1_0().create_instance(
            create_info,
            allocation_callbacks.as_raw_ptr(),
            &mut instance,
        );
        if err_code != vk::Result::SUCCESS {
            return Err(InstanceError::VkError(err_code));
        }
        Ok(Instance::load(&self.static_fn, instance))
    }
    fn fp_v1_0(&self) -> &vk::EntryFnV1_0 {
        &self.entry_fn_1_0
    }
    fn static_fn(&self) -> &vk::StaticFn {
        &self.static_fn
    }
}
