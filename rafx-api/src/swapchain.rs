#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxSwapchainDx12;
#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-dx12",
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2",
        feature = "rafx-gles3"
    ))
))]
use crate::empty::RafxSwapchainEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxSwapchainGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxSwapchainGles3;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSwapchainMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxSwapchainVulkan;
use crate::{
    RafxFence, RafxFormat, RafxResult, RafxSemaphore, RafxSwapchainColorSpace, RafxSwapchainDef,
    RafxSwapchainImage,
};

/// A set of images that act as a "backbuffer" of a window.
pub enum RafxSwapchain {
    #[cfg(feature = "rafx-dx12")]
    Dx12(RafxSwapchainDx12),
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxSwapchainVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSwapchainMetal),
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxSwapchainGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxSwapchainGles3),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    Empty(RafxSwapchainEmpty),
}

impl RafxSwapchain {
    /// Get the number of images in the swapchain. This is important to know because it indicates
    /// how many frames may be "in-flight" at a time - which affects how long a resource may be
    /// "in-use" after a command buffere referencing it has been submitted
    pub fn image_count(&self) -> usize {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => inner.image_count(),
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.image_count(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.image_count(),
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => inner.image_count(),
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => inner.image_count(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => inner.image_count(),
        }
    }

    /// Get the format of the images used in the swapchain
    pub fn format(&self) -> RafxFormat {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => inner.format(),
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.format(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.format(),
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => inner.format(),
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => inner.format(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => inner.format(),
        }
    }

    /// Get the color space of the images used in the swapchain
    pub fn color_space(&self) -> RafxSwapchainColorSpace {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => inner.color_space(),
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.color_space(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.color_space(),
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => inner.color_space(),
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => inner.color_space(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => inner.color_space(),
        }
    }

    /// Return the metadata used to create the swapchain
    pub fn swapchain_def(&self) -> &RafxSwapchainDef {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => inner.swapchain_def(),
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.swapchain_def(),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.swapchain_def(),
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => inner.swapchain_def(),
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => inner.swapchain_def(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => inner.swapchain_def(),
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
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => {
                inner.acquire_next_image_fence(fence.dx12_fence().unwrap())
            }
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.acquire_next_image_fence(fence.vk_fence().unwrap()),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => {
                inner.acquire_next_image_fence(fence.metal_fence().unwrap())
            }
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => {
                inner.acquire_next_image_fence(fence.gles2_fence().unwrap())
            }
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => {
                inner.acquire_next_image_fence(fence.gles3_fence().unwrap())
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => {
                inner.acquire_next_image_fence(fence.empty_fence().unwrap())
            }
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
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => {
                inner.acquire_next_image_semaphore(semaphore.dx12_semaphore().unwrap())
            }
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => {
                inner.acquire_next_image_semaphore(semaphore.vk_semaphore().unwrap())
            }
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => {
                inner.acquire_next_image_semaphore(semaphore.metal_semaphore().unwrap())
            }
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => {
                inner.acquire_next_image_semaphore(semaphore.gles2_semaphore().unwrap())
            }
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => {
                inner.acquire_next_image_semaphore(semaphore.gles3_semaphore().unwrap())
            }
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => {
                inner.acquire_next_image_semaphore(semaphore.empty_semaphore().unwrap())
            }
        }
    }

    /// Rebuild the swapchain. This is most commonly called when a window is resized.
    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => inner.rebuild(swapchain_def),
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => inner.rebuild(swapchain_def),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => inner.rebuild(swapchain_def),
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => inner.rebuild(swapchain_def),
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => inner.rebuild(swapchain_def),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => inner.rebuild(swapchain_def),
        }
    }

    /// Get the underlying dx12 API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_swapchain(&self) -> Option<&RafxSwapchainDx12> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(inner) => Some(inner),
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(_) => None,
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_swapchain(&self) -> Option<&RafxSwapchainVulkan> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_swapchain(&self) -> Option<&RafxSwapchainMetal> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_swapchain(&self) -> Option<&RafxSwapchainGles2> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_swapchain(&self) -> Option<&RafxSwapchainGles3> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-dx12",
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    pub fn empty_swapchain(&self) -> Option<&RafxSwapchainEmpty> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSwapchain::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSwapchain::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSwapchain::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSwapchain::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSwapchain::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-dx12",
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxSwapchain::Empty(inner) => Some(inner),
        }
    }
}
