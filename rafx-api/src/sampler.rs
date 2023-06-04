#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxSamplerDx12;
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
use crate::empty::RafxSamplerEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxSamplerGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxSamplerGles3;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxSamplerMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxSamplerVulkan;

/// Configures how images will be sampled by the GPU
///
/// Samplers must not be dropped if they are in use by the GPU
#[derive(Debug, Clone)]
pub enum RafxSampler {
    #[cfg(feature = "rafx-dx12")]
    Dx12(RafxSamplerDx12),
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxSamplerVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxSamplerMetal),
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxSamplerGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxSamplerGles3),
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
    Empty(RafxSamplerEmpty),
}

impl RafxSampler {
    /// Get the underlying dx12 API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_sampler(&self) -> Option<&RafxSamplerDx12> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSampler::Dx12(inner) => Some(inner),
            #[cfg(feature = "rafx-vulkan")]
            RafxSampler::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSampler::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSampler::Gles3(_) => None,
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
            RafxSampler::Empty(_) => None,
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_sampler(&self) -> Option<&RafxSamplerVulkan> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSampler::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSampler::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSampler::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSampler::Gles3(_) => None,
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
            RafxSampler::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_sampler(&self) -> Option<&RafxSamplerMetal> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSampler::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSampler::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxSampler::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSampler::Gles3(_) => None,
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
            RafxSampler::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_sampler(&self) -> Option<&RafxSamplerGles2> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSampler::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSampler::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSampler::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxSampler::Gles3(_) => None,
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
            RafxSampler::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_sampler(&self) -> Option<&RafxSamplerGles3> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSampler::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSampler::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSampler::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSampler::Gles3(inner) => Some(inner),
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
            RafxSampler::Empty(_) => None,
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
    pub fn empty_sampler(&self) -> Option<&RafxSamplerEmpty> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxSampler::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxSampler::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxSampler::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxSampler::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxSampler::Gles3(_) => None,
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
            RafxSampler::Empty(inner) => Some(inner),
        }
    }
}
