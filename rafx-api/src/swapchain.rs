#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSwapchainMetal;
use crate::vulkan::RafxSwapchainVulkan;
use crate::{
    RafxFence, RafxFormat, RafxResult, RafxSemaphore, RafxSwapchainDef, RafxSwapchainImage,
};

pub enum RafxSwapchain {
    Vk(RafxSwapchainVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSwapchainMetal),
}

impl RafxSwapchain {
    pub fn image_count(&self) -> usize {
        match self {
            RafxSwapchain::Vk(inner) => inner.image_count(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => unimplemented!(),
        }
    }

    pub fn format(&self) -> RafxFormat {
        match self {
            RafxSwapchain::Vk(inner) => inner.format(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => unimplemented!(),
        }
    }

    pub fn swapchain_def(&self) -> &RafxSwapchainDef {
        match self {
            RafxSwapchain::Vk(inner) => inner.swapchain_def(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => unimplemented!(),
        }
    }

    pub fn acquire_next_image_fence(
        &mut self,
        fence: &RafxFence,
    ) -> RafxResult<RafxSwapchainImage> {
        match self {
            RafxSwapchain::Vk(inner) => inner.acquire_next_image_fence(fence.vk_fence().unwrap()),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => unimplemented!(),
        }
    }

    pub fn acquire_next_image_semaphore(
        &mut self,
        semaphore: &RafxSemaphore,
    ) -> RafxResult<RafxSwapchainImage> {
        match self {
            RafxSwapchain::Vk(inner) => {
                inner.acquire_next_image_semaphore(semaphore.vk_semaphore().unwrap())
            }
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => unimplemented!(),
        }
    }

    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        match self {
            RafxSwapchain::Vk(inner) => inner.rebuild(swapchain_def),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => unimplemented!(),
        }
    }

    pub fn vk_swapchain(&self) -> Option<&RafxSwapchainVulkan> {
        match self {
            RafxSwapchain::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_swapchain(&self) -> Option<&RafxSwapchainMetal> {
        match self {
            RafxSwapchain::Vk(_) => None,
            RafxSwapchain::Metal(inner) => Some(inner),
        }
    }
}
