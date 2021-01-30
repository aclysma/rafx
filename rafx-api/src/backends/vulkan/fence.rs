use crate::vulkan::RafxDeviceContextVulkan;
use crate::{RafxFenceStatus, RafxResult};
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxFenceVulkan {
    device_context: RafxDeviceContextVulkan,
    vk_fence: vk::Fence,
    // Set to true when an operation is scheduled to signal this fence
    // Cleared when an operation is scheduled to consume this fence
    submitted: AtomicBool,
}

impl Drop for RafxFenceVulkan {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_fence(self.vk_fence, None)
        }
    }
}

impl RafxFenceVulkan {
    pub fn new(device_context: &RafxDeviceContextVulkan) -> RafxResult<RafxFenceVulkan> {
        let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::empty());

        let vk_fence = unsafe { device_context.device().create_fence(&*create_info, None)? };

        Ok(RafxFenceVulkan {
            device_context: device_context.clone(),
            vk_fence,
            submitted: AtomicBool::new(false),
        })
    }

    pub fn vk_fence(&self) -> vk::Fence {
        self.vk_fence
    }

    pub(crate) fn submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn wait(&self) -> RafxResult<()> {
        Self::wait_for_fences(&self.device_context, &[self])
    }

    pub fn wait_for_fences(
        device_context: &RafxDeviceContextVulkan,
        fences: &[&RafxFenceVulkan],
    ) -> RafxResult<()> {
        let mut fence_list = Vec::with_capacity(fences.len());
        for fence in fences {
            if fence.submitted() {
                fence_list.push(fence.vk_fence());
            }
        }

        if !fence_list.is_empty() {
            let device = device_context.device();
            unsafe {
                device.wait_for_fences(&fence_list, true, std::u64::MAX)?;
                device.reset_fences(&fence_list)?;
            }
        }

        for fence in fences {
            fence.set_submitted(false);
        }

        Ok(())
    }

    pub(crate) fn set_submitted(
        &self,
        available: bool,
    ) {
        self.submitted.store(available, Ordering::Relaxed);
    }

    pub fn get_fence_status(&self) -> RafxResult<RafxFenceStatus> {
        if !self.submitted() {
            Ok(RafxFenceStatus::Unsubmitted)
        } else {
            let device = self.device_context.device();
            unsafe {
                let is_ready = device.get_fence_status(self.vk_fence)?;
                if is_ready {
                    device.reset_fences(&[self.vk_fence])?;
                    self.set_submitted(false);
                }

                if is_ready {
                    Ok(RafxFenceStatus::Complete)
                } else {
                    Ok(RafxFenceStatus::Incomplete)
                }
            }
        }
    }
}
