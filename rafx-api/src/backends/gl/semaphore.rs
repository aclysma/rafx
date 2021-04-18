use crate::gl::RafxDeviceContextGl;
use crate::RafxResult;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxSemaphoreGl {
    device_context: RafxDeviceContextGl,

    // Set to true when an operation is scheduled to signal this semaphore
    // Cleared when an operation is scheduled to consume this semaphore
    signal_available: AtomicBool,
}

impl RafxSemaphoreGl {
    pub fn new(device_context: &RafxDeviceContextGl) -> RafxResult<RafxSemaphoreGl> {
        // Semaphores are not available on OpenGL ES 2.0
        // use glFlush for Gpu->Gpu sync
        Ok(RafxSemaphoreGl {
            device_context: device_context.clone(),
            signal_available: AtomicBool::new(false),
        })
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

    pub(crate) fn handle_wait_semaphores(semaphores: &[&RafxSemaphoreGl]) -> RafxResult<()> {
        if semaphores.is_empty() {
            return Ok(());
        }

        let mut should_flush = false;
        for &semaphore in semaphores {
            if semaphore.signal_available() {
                should_flush = true;
                semaphore.set_signal_available(false);
            }
        }

        if should_flush {
            if let Some(first) = semaphores.first() {
                first.device_context.gl_context().gl_flush()?;
            }
        }

        Ok(())
    }
}
