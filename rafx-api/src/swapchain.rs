#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSwapchainMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxSwapchainVulkan;
use crate::{
    RafxFence, RafxFormat, RafxResult, RafxSemaphore, RafxSwapchainDef, RafxSwapchainImage,
};

/// A set of images that act as a "backbuffer" of a window.
pub enum RafxSwapchain {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxSwapchainVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSwapchainMetal),
}

impl RafxSwapchain {
    /// Get the number of images in the swapchain. This is important to know because it indicates
    /// how many frames may be "in-flight" at a time - which affects how long a resource may be
    /// "in-use" after a command buffere referencing it has been submitted
    pub fn image_count(&self) -> usize {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.image_count(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.image_count(),
        }
    }

    /// Get the format of the images used in the swapchain
    pub fn format(&self) -> RafxFormat {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.format(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.format(),
        }
    }

    /// Return the metadata used to create the swapchain
    pub fn swapchain_def(&self) -> &RafxSwapchainDef {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.swapchain_def(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.swapchain_def(),
        }
    }

    /// Acquire the next image. The given fence will be signaled when it is available
    ///
    /// This is the same as `acquire_next_image_semaphore` except that it signals a fence.
    pub fn acquire_next_image_fence(
        &mut self,
        fence: &RafxFence,
    ) -> RafxResult<RafxSwapchainImage> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.acquire_next_image_fence(fence.vk_fence().unwrap()),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.acquire_next_image(),
        }
    }

    /// Acquire the next image. The given semaphore will be signaled when it is available
    ///
    /// This is the same as `acquire_next_image_fence` except that it signals a semaphore.
    pub fn acquire_next_image_semaphore(
        &mut self,
        semaphore: &RafxSemaphore,
    ) -> RafxResult<RafxSwapchainImage> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => {
                inner.acquire_next_image_semaphore(semaphore.vk_semaphore().unwrap())
            }
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.acquire_next_image(),
        }
    }

    /// Rebuild the swapchain. This is most commonly called when a window is resized.
    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.rebuild(swapchain_def),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.rebuild(swapchain_def),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_swapchain(&self) -> Option<&RafxSwapchainVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_swapchain(&self) -> Option<&RafxSwapchainMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => Some(inner),
        }
    }
}
