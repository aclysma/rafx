use crate::vulkan::RafxDeviceContextVulkan;
use crate::RafxResult;
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxSemaphoreVulkan {
    device_context: RafxDeviceContextVulkan,
    vk_semaphore: vk::Semaphore,
    // Set to true when an operation is scheduled to signal this semaphore
    // Cleared when an operation is scheduled to consume this semaphore
    signal_available: AtomicBool,
}

impl Drop for RafxSemaphoreVulkan {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_semaphore(self.vk_semaphore, None)
        }
    }
}

impl RafxSemaphoreVulkan {
    pub fn new(device_context: &RafxDeviceContextVulkan) -> RafxResult<RafxSemaphoreVulkan> {
        let create_info =
            vk::SemaphoreCreateInfo::builder().flags(vk::SemaphoreCreateFlags::empty());

        let vk_semaphore = unsafe {
            device_context
                .device()
                .create_semaphore(&*create_info, None)?
        };

        Ok(RafxSemaphoreVulkan {
            device_context: device_context.clone(),
            vk_semaphore,
            signal_available: AtomicBool::new(false),
        })
    }

    pub fn vk_semaphore(&self) -> vk::Semaphore {
        self.vk_semaphore
    }

    pub(crate) fn signal_available(&self) -> bool {
        self.signal_available.load(Ordering::Relaxed)
    }

    pub(crate) fn set_signal_available(
        &self,
        available: bool,
    ) {
        self.signal_available.store(available, Ordering::Relaxed);
    }
}
