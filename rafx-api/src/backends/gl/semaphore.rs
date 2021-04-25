use crate::gl::RafxDeviceContextGles2;
use crate::RafxResult;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxSemaphoreGles2 {
    _device_context: RafxDeviceContextGles2,

    // Set to true when an operation is scheduled to signal this semaphore
    // Cleared when an operation is scheduled to consume this semaphore
    signal_available: AtomicBool,
}

impl RafxSemaphoreGles2 {
    pub fn new(device_context: &RafxDeviceContextGles2) -> RafxResult<RafxSemaphoreGles2> {
        // Semaphores are not available on OpenGL ES 2.0
        // use glFlush for Gpu->Gpu sync
        Ok(RafxSemaphoreGles2 {
            _device_context: device_context.clone(),
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
}
