use crate::gles3::RafxDeviceContextGles3;
use crate::{RafxFenceStatus, RafxResult};
use rafx_base::trust_cell::TrustCell;
use std::sync::atomic::{AtomicBool, Ordering};

//TODO: GL ES 3.0 has some sync primitives

pub struct RafxFenceGles3 {
    device_context: RafxDeviceContextGles3,
    // Set to true when an operation is scheduled to signal this fence
    // Cleared when an operation is scheduled to consume this fence
    submitted: AtomicBool,
    gl_finish_call_count: TrustCell<u64>,
}

impl RafxFenceGles3 {
    pub fn new(device_context: &RafxDeviceContextGles3) -> RafxResult<RafxFenceGles3> {
        // Fences are not available on OpenGL ES 2.0
        // use glFlush for Gpu->Cpu sync
        Ok(RafxFenceGles3 {
            device_context: device_context.clone(),
            submitted: AtomicBool::new(false),
            gl_finish_call_count: Default::default(),
        })
    }

    pub(crate) fn submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn wait(&self) -> RafxResult<()> {
        if self.submitted() {
            self.device_context.gl_finish()?;
        }

        self.set_submitted(false);
        Ok(())
    }

    pub fn wait_for_fences(
        _device_context: &RafxDeviceContextGles3,
        fences: &[&RafxFenceGles3],
    ) -> RafxResult<()> {
        let mut finish_called = false;
        for fence in fences {
            if fence.submitted() {
                fence.device_context.gl_finish()?;
                finish_called = true;
                break;
            }
        }

        if finish_called {
            for fence in fences {
                fence.set_submitted(false);
            }
        }

        Ok(())
    }

    pub(crate) fn set_submitted(
        &self,
        available: bool,
    ) {
        if available {
            // Set the call count to the global device count. If it increments past the cached count,
            // we will know that finish was called since this fence was submitted
            *self.gl_finish_call_count.borrow_mut() = self
                .device_context
                .inner
                .gl_finish_call_count
                .load(Ordering::Relaxed);
        }
        self.submitted.store(available, Ordering::Relaxed);
    }

    pub fn get_fence_status(&self) -> RafxResult<RafxFenceStatus> {
        if !self.submitted() {
            Ok(RafxFenceStatus::Unsubmitted)
        } else {
            if *self.gl_finish_call_count.borrow()
                >= self
                    .device_context
                    .inner
                    .gl_finish_call_count
                    .load(Ordering::Relaxed)
            {
                self.set_submitted(false);
                Ok(RafxFenceStatus::Complete)
            } else {
                Ok(RafxFenceStatus::Incomplete)
            }
        }
    }
}
