#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2"
    ))
))]
use crate::empty::RafxRootSignatureEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxRootSignatureGles2;
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
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxRootSignatureGles2),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2"
        ))
    ))]
    Empty(RafxRootSignatureEmpty),
}

impl RafxRootSignature {
    /// Returns what kind of pipeline this is
    pub fn pipeline_type(&self) -> RafxPipelineType {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(inner) => inner.pipeline_type(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxRootSignature::Empty(inner) => inner.pipeline_type(),
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
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxRootSignature::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_root_signature(&self) -> Option<&RafxRootSignatureMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxRootSignature::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_root_signature(&self) -> Option<&RafxRootSignatureGles2> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxRootSignature::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2"
        ))
    ))]
    pub fn empty_root_signature(&self) -> Option<&RafxRootSignatureEmpty> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxRootSignature::Empty(inner) => Some(inner),
        }
    }
}
