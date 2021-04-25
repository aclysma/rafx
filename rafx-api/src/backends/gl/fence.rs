use crate::gl::RafxDeviceContextGl;
use crate::{RafxFenceStatus, RafxResult};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxFenceGl {
    device_context: RafxDeviceContextGl,
    // Set to true when an operation is scheduled to signal this fence
    // Cleared when an operation is scheduled to consume this fence
    submitted: AtomicBool,
}

impl RafxFenceGl {
    pub fn new(device_context: &RafxDeviceContextGl) -> RafxResult<RafxFenceGl> {
        // Fences are not available on OpenGL ES 2.0
        // use glFlush for Gpu->Cpu sync
        Ok(RafxFenceGl {
            device_context: device_context.clone(),
            submitted: AtomicBool::new(false),
        })
    }

    pub(crate) fn submitted(&self) -> bool {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn wait(&self) -> RafxResult<()> {
        if self.submitted() {
            self.device_context.gl_context().gl_finish()?;
        }

        self.set_submitted(false);
        Ok(())
    }

    pub fn wait_for_fences(
        _device_context: &RafxDeviceContextGl,
        fences: &[&RafxFenceGl],
    ) -> RafxResult<()> {
        let mut finish_called = false;
        for fence in fences {
            if fence.submitted() {
                fence.device_context.gl_context().gl_finish()?;
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
        self.submitted.store(available, Ordering::Relaxed);
    }

    pub fn get_fence_status(&self) -> RafxResult<RafxFenceStatus> {
        if !self.submitted() {
            Ok(RafxFenceStatus::Unsubmitted)
        } else {
            Ok(RafxFenceStatus::Incomplete)
        }
    }
}
