use super::{RafxDeviceContextDx12, RafxFenceDx12};
use crate::RafxResult;
use std::sync::atomic::{AtomicBool, Ordering};

// Dx12 just has fences
pub struct RafxSemaphoreDx12 {
    fence: RafxFenceDx12,
    // Set to true when an operation is scheduled to signal this semaphore
    // Cleared when an operation is scheduled to consume this semaphore
    signal_available: AtomicBool,
}

impl RafxSemaphoreDx12 {
    pub fn new(device_context: &RafxDeviceContextDx12) -> RafxResult<RafxSemaphoreDx12> {
        //TODO: Need to add support for new_event() in metal crate

        let fence = RafxFenceDx12::new(device_context)?;

        Ok(RafxSemaphoreDx12 {
            fence,
            signal_available: AtomicBool::new(false),
        })
    }

    pub fn fence(&self) -> &RafxFenceDx12 {
        &self.fence
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
