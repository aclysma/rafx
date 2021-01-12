#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSemaphoreMetal;
use crate::vulkan::RafxSemaphoreVulkan;

pub enum RafxSemaphore {
    Vk(RafxSemaphoreVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSemaphoreMetal),
}

impl RafxSemaphore {
    pub fn vk_semaphore(&self) -> Option<&RafxSemaphoreVulkan> {
        match self {
            RafxSemaphore::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSemaphore::Metal(_) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_semaphore(&self) -> Option<&RafxSemaphoreMetal> {
        match self {
            RafxSemaphore::Vk(_) => None,
            RafxSemaphore::Metal(inner) => Some(inner),
        }
    }
}
