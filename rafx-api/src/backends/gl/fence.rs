use crate::gl::RafxDeviceContextGl;
use crate::{RafxFenceStatus, RafxResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct RafxFenceGl {
    _device_context: RafxDeviceContextGl,
    //mtl_semaphore: Arc<dispatch::Semaphore>,
    // Set to true when an operation is scheduled to signal this fence
    // Cleared when an operation is scheduled to consume this fence
    submitted: AtomicBool,
}

impl RafxFenceGl {
    pub fn new(device_context: &RafxDeviceContextGl) -> RafxResult<RafxFenceGl> {
        //let mtl_semaphore = dispatch::Semaphore::new(0);

        Ok(RafxFenceGl {
            _device_context: device_context.clone(),
            //mtl_semaphore: Arc::new(mtl_semaphore),
            submitted: AtomicBool::new(false),
        })
    }

    // pub(crate) fn metal_dispatch_semaphore(&self) -> &Arc<dispatch::Semaphore> {
    //     &self.mtl_semaphore
    // }

    pub(crate) fn submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn wait(&self) -> RafxResult<()> {
        if self.submitted() {
            //self.mtl_semaphore.wait();
        }

        self.set_submitted(false);
        Ok(())
    }

    pub fn wait_for_fences(
        _device_context: &RafxDeviceContextGl,
        fences: &[&RafxFenceGl],
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
            Ok(RafxFenceStatus::Unsubmitted)
        } else {
            // let is_ready = self
            //     .mtl_semaphore
            //     .wait_timeout(std::time::Duration::default())
            //     .is_ok();
            let is_ready = true;
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
