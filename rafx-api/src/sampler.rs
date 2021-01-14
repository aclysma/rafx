#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSamplerMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxSamplerVulkan;

/// Configures how images will be sampled by the GPU
///
/// Samplers must not be dropped if they are in use by the GPU
#[derive(Debug, Clone)]
pub enum RafxSampler {
    Vk(RafxSamplerVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSamplerMetal),
}

impl RafxSampler {
    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_sampler(&self) -> Option<&RafxSamplerVulkan> {
        match self {
            RafxSampler::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_sampler(&self) -> Option<&RafxSamplerMetal> {
        match self {
            RafxSampler::Vk(_inner) => None,
            RafxSampler::Metal(inner) => Some(inner),
        }
    }
}
