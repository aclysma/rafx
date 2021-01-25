#[cfg(feature = "rafx-metal")]
use crate::metal::RafxRootSignatureMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxRootSignatureVulkan;
use crate::RafxPipelineType;

/// Represents the full "layout" or "interface" of a shader (or set of shaders.)
///
/// A root signature is created from shader metadata that can be manually supplied or generated via
/// reflection.
#[derive(Clone, Debug)]
pub enum RafxRootSignature {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxRootSignatureVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxRootSignatureMetal),
}

impl RafxRootSignature {
    /// Returns what kind of pipeline this is
    pub fn pipeline_type(&self) -> RafxPipelineType {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => inner.pipeline_type(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_root_signature(&self) -> Option<&RafxRootSignatureVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_root_signature(&self) -> Option<&RafxRootSignatureMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => Some(inner),
        }
    }
}
