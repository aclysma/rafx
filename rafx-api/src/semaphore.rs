#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxSemaphoreDx12;
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
use crate::empty::RafxSemaphoreEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxSemaphoreGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxSemaphoreGles3;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSemaphoreMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxSemaphoreVulkan;

/// A GPU -> GPU synchronization mechanism.
///
/// A semaphore is either "signalled" or "unsignalled". Only the GPU can read or write this status.
///
/// Semaphores can be used to queue multiple dependent units of work to the GPU where one unit of
/// work cannot start until another unit of work completes.
///
/// Semaphores must not be dropped if they are in use by the GPU.
pub enum RafxSemaphore {
    #[cfg(feature = "rafx-dx12")]
    Dx12(RafxSemaphoreDx12),
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxSemaphoreVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSemaphoreMetal),
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxSemaphoreGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxSemaphoreGles3),
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
    Empty(RafxSemaphoreEmpty),
}

impl RafxSemaphore {
    /// Get the underlying dx12 API object. This provides access to any internally created
    /// dx12 objects.
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_semaphore(&self) -> Option<&RafxSemaphoreDx12> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSemaphore::Dx12(inner) => Some(inner),
            #[cfg(feature = "rafx-vulkan")]
            RafxSemaphore::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSemaphore::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSemaphore::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSemaphore::Gles3(_) => None,
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
            RafxSemaphore::Empty(_) => None,
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_semaphore(&self) -> Option<&RafxSemaphoreVulkan> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSemaphore::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSemaphore::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSemaphore::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSemaphore::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSemaphore::Gles3(_) => None,
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
            RafxSemaphore::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_semaphore(&self) -> Option<&RafxSemaphoreMetal> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSemaphore::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSemaphore::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSemaphore::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxSemaphore::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSemaphore::Gles3(_) => None,
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
            RafxSemaphore::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_semaphore(&self) -> Option<&RafxSemaphoreGles2> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSemaphore::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSemaphore::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSemaphore::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSemaphore::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxSemaphore::Gles3(_) => None,
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
            RafxSemaphore::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_semaphore(&self) -> Option<&RafxSemaphoreGles3> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSemaphore::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSemaphore::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSemaphore::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSemaphore::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSemaphore::Gles3(inner) => Some(inner),
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
            RafxSemaphore::Empty(_) => None,
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
    pub fn empty_semaphore(&self) -> Option<&RafxSemaphoreEmpty> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSemaphore::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSemaphore::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSemaphore::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSemaphore::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSemaphore::Gles3(_) => None,
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
            RafxSemaphore::Empty(inner) => Some(inner),
        }
    }
}
