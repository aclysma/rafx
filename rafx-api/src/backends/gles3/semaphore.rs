use crate::gles3::RafxDeviceContextGles3;
use crate::RafxResult;
use std::sync::atomic::{AtomicBool, Ordering};

//TODO: GL ES 3.0 has some sync primitives

pub struct RafxSemaphoreGles3 {
    _device_context: RafxDeviceContextGles3,

    // Set to true when an operation is scheduled to signal this semaphore
    // Cleared when an operation is scheduled to consume this semaphore
    signal_available: AtomicBool,
}

impl RafxSemaphoreGles3 {
    pub fn new(device_context: &RafxDeviceContextGles3) -> RafxResult<RafxSemaphoreGles3> {
        // Semaphores are not available on OpenGL ES 2.0
        // use glFlush for Gpu->Gpu sync
        Ok(RafxSemaphoreGles3 {
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
