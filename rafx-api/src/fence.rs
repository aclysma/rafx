#[cfg(feature = "rafx-metal")]
use crate::metal::RafxFenceMetal;
use crate::vulkan::RafxFenceVulkan;
use crate::{RafxFenceStatus, RafxResult};

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
