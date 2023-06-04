use crate::dx12::{RafxDeviceContextDx12, RafxQueueDx12};
use crate::{RafxFenceStatus, RafxResult};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Debug)]
pub struct RafxFenceDx12 {
    _device_context: RafxDeviceContextDx12,
    fence: super::d3d12::ID3D12Fence1,
    wait_idle_fence_event: windows::Win32::Foundation::HANDLE,
    // Set to true when an operation is scheduled to signal this fence
    // Cleared when an operation is scheduled to consume this fence
    submitted: AtomicBool,

    // After the fence is signaled with this value, we increment. We check the fence is complete
    // by waiting for fence_value - 1.
    fence_value: AtomicU64,
}

impl RafxFenceDx12 {
    pub fn new(device_context: &RafxDeviceContextDx12) -> RafxResult<RafxFenceDx12> {
        let fence = unsafe {
            device_context
                .d3d12_device()
                .CreateFence(0, super::d3d12::D3D12_FENCE_FLAGS::default())
        }?;

        let wait_idle_fence_event =
            unsafe { windows::Win32::System::Threading::CreateEventW(None, false, false, None) }?;

        Ok(RafxFenceDx12 {
            _device_context: device_context.clone(),
            fence,
            wait_idle_fence_event,
            submitted: AtomicBool::new(false),
            fence_value: AtomicU64::new(1),
        })
    }

    pub fn dx12_fence(&self) -> &super::d3d12::ID3D12Fence1 {
        &self.fence
    }

    pub(crate) fn submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn fence_value(&self) -> &AtomicU64 {
        &self.fence_value
    }

    // Have the queue send another signal, the signaled value is returned
    pub fn queue_signal(
        &self,
        queue: &RafxQueueDx12,
    ) -> RafxResult<u64> {
        self.set_submitted(true);
        let signal_value = self.fence_value.fetch_add(1, Ordering::Relaxed);
        unsafe {
            //println!("Signaling value {} on {:?}", signal_value, self.fence);
            queue.dx12_queue().Signal(&self.fence, signal_value)?;
        }
        Ok(signal_value)
    }

    // Have the queue wait for the most recently sent signal
    pub fn queue_wait(
        &self,
        queue: &RafxQueueDx12,
    ) -> RafxResult<()> {
        unsafe {
            let wait_value = self.fence_value.load(Ordering::Relaxed) - 1;
            //println!("Queue GPU-side wait for value {} on {:?}", wait_value, self.fence);
            queue.dx12_queue().Wait(&self.fence, wait_value)?;
        }

        Ok(())
    }

    pub fn wait(&self) -> RafxResult<()> {
        //println!("wait for fence {:?}", self.fence);
        let fence_status = self.get_fence_status()?;
        // We don not wait on fences that have never been submitted
        if fence_status == RafxFenceStatus::Incomplete {
            unsafe {
                // Need to ResetEvent here? Not Sure
                let fence_value = self.fence_value.load(Ordering::Relaxed) - 1;
                //log::info!("wait for event {} on {:?}", fence_value, self.fence);
                windows::Win32::System::Threading::ResetEvent(self.wait_idle_fence_event);
                self.fence
                    .SetEventOnCompletion(fence_value, self.wait_idle_fence_event)?;
                const INFINITE: u32 = u32::MAX;
                windows::Win32::System::Threading::WaitForSingleObject(
                    self.wait_idle_fence_event,
                    INFINITE,
                );
            }
        }

        self.set_submitted(false);
        Ok(())
    }

    pub fn wait_for_fences(
        _device_context: &RafxDeviceContextDx12,
        fences: &[&RafxFenceDx12],
    ) -> RafxResult<()> {
        for fence in fences {
            if fence.submitted() {
                fence.wait()?;
            }
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
            //println!("fence never submitted, no wait");
            Ok(RafxFenceStatus::Unsubmitted)
        } else {
            let is_ready = unsafe {
                let completed_value = self.fence.GetCompletedValue();
                //println!("completed {} value {} on {:?}", completed_value, self.fence_value.load(Ordering::Relaxed) - 1, self.fence);
                completed_value >= self.fence_value.load(Ordering::Relaxed) - 1
            };
            if is_ready {
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
