#[cfg(feature = "rafx-metal")]
use crate::metal::RafxFenceMetal;
use crate::vulkan::RafxFenceVulkan;
use crate::{RafxFenceStatus, RafxResult};

/// A GPU -> CPU synchronization mechanism.
///
/// A fence can be in the following states:
///  * Unsubmitted - Initial state when created
///  * Incomplete - Once a command buffer is submitted, the fence is marked as incomplete
///  * Complete - The GPU can mark a fence as complete to signal completion of work.
///
/// The status of the fence returns to Unsubmitted when get_fence_status() is called while in a
/// completed state. In other words, the Complete status can only be returned one time unless the
/// fence is submitted again.
///
/// Fences must not be dropped if they are in use by the GPU.
pub enum RafxFence {
    Vk(RafxFenceVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxFenceMetal),
}

impl RafxFence {
    pub fn get_fence_status(&self) -> RafxResult<RafxFenceStatus> {
        match self {
            RafxFence::Vk(inner) => inner.get_fence_status(),
            #[cfg(feature = "rafx-metal")]
            RafxFence::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn wait(&self) -> RafxResult<()> {
        match self {
            RafxFence::Vk(inner) => inner.wait(),
            #[cfg(feature = "rafx-metal")]
            RafxFence::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_fence(&self) -> Option<&RafxFenceVulkan> {
        match self {
            RafxFence::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxFence::Metal(_) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_fence(&self) -> Option<&RafxFenceMetal> {
        match self {
            RafxFence::Vk(_) => None,
            RafxFence::Metal(inner) => Some(inner),
        }
    }
}
