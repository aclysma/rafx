use crate::gl::RafxDeviceContextGl;
use crate::RafxResult;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct RafxSemaphoreGl {
    _device_context: RafxDeviceContextGl,

    // Set to true when an operation is scheduled to signal this semaphore
    // Cleared when an operation is scheduled to consume this semaphore
    signal_available: AtomicBool,

    //gl_event: gl_rs::Event,
}

// for gl_rs::Event
unsafe impl Send for RafxSemaphoreGl {}
unsafe impl Sync for RafxSemaphoreGl {}

impl RafxSemaphoreGl {
    pub fn new(device_context: &RafxDeviceContextGl) -> RafxResult<RafxSemaphoreGl> {
        unimplemented!();
        // //TODO: Need to add support for new_event() in gl crate
        //
        // let gl_event = device_context.device().new_event();
        //
        // Ok(RafxSemaphoreGl {
        //     _device_context: device_context.clone(),
        //     gl_event,
        //     signal_available: AtomicBool::new(false),
        // })
    }

    // pub fn gl_event(&self) -> &gl_rs::EventRef {
    //     self.gl_event.as_ref()
    // }

    pub(crate) fn signal_available(&self) -> bool {
        unimplemented!();
        self.signal_available.load(Ordering::Relaxed)
    }

    pub(crate) fn set_signal_available(
        &self,
        available: bool,
    ) {
        unimplemented!();
        self.signal_available.store(available, Ordering::Relaxed);
    }
}
