use crate::gles2::RafxDeviceContextGles2;
use crate::{RafxFenceStatus, RafxResult};
use std::sync::atomic::{AtomicBool, Ordering};

//TODO: GL ES 3.0 has some sync primitives

pub struct RafxFenceGles2 {
    device_context: RafxDeviceContextGles2,
    // Set to true when an operation is scheduled to signal this fence
    // Cleared when an operation is scheduled to consume this fence
    submitted: AtomicBool,
}

impl RafxFenceGles2 {
    pub fn new(device_context: &RafxDeviceContextGles2) -> RafxResult<RafxFenceGles2> {
        // Fences are not available on OpenGL ES 2.0
        // use glFlush for Gpu->Cpu sync
        Ok(RafxFenceGles2 {
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
        _device_context: &RafxDeviceContextGles2,
        fences: &[&RafxFenceGles2],
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
