#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSamplerMetal;
use crate::vulkan::RafxSamplerVulkan;

#[derive(Debug, Clone)]
pub enum RafxSampler {
    Vk(RafxSamplerVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSamplerMetal),
}

impl RafxSampler {
    pub fn vk_sampler(&self) -> Option<&RafxSamplerVulkan> {
        match self {
            RafxSampler::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_sampler(&self) -> Option<&RafxSamplerMetal> {
        match self {
            RafxSampler::Vk(_inner) => None,
            RafxSampler::Metal(inner) => Some(inner),
        }
    }
}
