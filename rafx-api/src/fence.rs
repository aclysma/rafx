#[cfg(feature = "rafx-metal")]
use crate::metal::RafxFenceMetal;
#[cfg(feature = "rafx-vulkan")]
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
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxFenceVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxFenceMetal),
}

impl RafxFence {
    /// Get the status of the fence. See `RafxFenceStatus`
    ///
    /// The status of the fence returns to Unsubmitted when get_fence_status() is called while in a
    /// completed state. In other words, the Complete status can only be returned one time unless the
    /// fence is submitted again.
    pub fn get_fence_status(&self) -> RafxResult<RafxFenceStatus> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxFence::Vk(inner) => inner.get_fence_status(),
            #[cfg(feature = "rafx-metal")]
            RafxFence::Metal(inner) => inner.get_fence_status(),
        }
    }

    /// Wait for the fence to be signaled as complete by the GPU
    pub fn wait(&self) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxFence::Vk(inner) => inner.wait(),
            #[cfg(feature = "rafx-metal")]
            RafxFence::Metal(inner) => inner.wait(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_fence(&self) -> Option<&RafxFenceVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxFence::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxFence::Metal(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_fence(&self) -> Option<&RafxFenceMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxFence::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxFence::Metal(inner) => Some(inner),
        }
    }
}
