#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxRootSignatureDx12;
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
use crate::empty::RafxRootSignatureEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxRootSignatureGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxRootSignatureGles3;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxRootSignatureMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxRootSignatureVulkan;
use crate::{RafxDescriptorIndex, RafxPipelineType, RafxShaderStageFlags};

/// Represents the full "layout" or "interface" of a shader (or set of shaders.)
///
/// A root signature is created from shader metadata that can be manually supplied or generated via
/// reflection.
#[derive(Clone, Debug)]
pub enum RafxRootSignature {
    #[cfg(feature = "rafx-dx12")]
    Dx12(RafxRootSignatureDx12),
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxRootSignatureVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxRootSignatureMetal),
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxRootSignatureGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxRootSignatureGles3),
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
    Empty(RafxRootSignatureEmpty),
}

impl RafxRootSignature {
    /// Returns what kind of pipeline this is
    pub fn pipeline_type(&self) -> RafxPipelineType {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(inner) => inner.pipeline_type(),
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
            RafxRootSignature::Empty(inner) => inner.pipeline_type(),
        }
    }

    pub fn find_descriptor_by_name(
        &self,
        name: &str,
    ) -> Option<RafxDescriptorIndex> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(inner) => inner.find_descriptor_by_name(name),
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => inner.find_descriptor_by_name(name),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => inner.find_descriptor_by_name(name),
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(inner) => inner.find_descriptor_by_name(name),
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(inner) => inner.find_descriptor_by_name(name),
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
            RafxRootSignature::Empty(inner) => inner.find_descriptor_by_name(name),
        }
    }

    pub fn find_descriptor_by_binding(
        &self,
        set_index: u32,
        binding: u32,
    ) -> Option<RafxDescriptorIndex> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(inner) => inner.find_descriptor_by_binding(set_index, binding),
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => inner.find_descriptor_by_binding(set_index, binding),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => inner.find_descriptor_by_binding(set_index, binding),
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(inner) => inner.find_descriptor_by_binding(set_index, binding),
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(inner) => inner.find_descriptor_by_binding(set_index, binding),
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
            RafxRootSignature::Empty(inner) => inner.find_descriptor_by_binding(set_index, binding),
        }
    }

    pub fn find_push_constant_descriptor(
        &self,
        stage: RafxShaderStageFlags,
    ) -> Option<RafxDescriptorIndex> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(inner) => inner.find_push_constant_descriptor(stage),
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => inner.find_push_constant_descriptor(stage),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => inner.find_push_constant_descriptor(stage),
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_inner) => {
                let _ = stage;
                unimplemented!()
            }
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(_inner) => {
                let _ = stage;
                unimplemented!()
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
            RafxRootSignature::Empty(inner) => inner.find_push_constant_descriptor(stage),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_root_signature(&self) -> Option<&RafxRootSignatureDx12> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(inner) => Some(inner),
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(_) => None,
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
            RafxRootSignature::Empty(_) => None,
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_root_signature(&self) -> Option<&RafxRootSignatureVulkan> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(_) => None,
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
            RafxRootSignature::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_root_signature(&self) -> Option<&RafxRootSignatureMetal> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(_) => None,
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
            RafxRootSignature::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_root_signature(&self) -> Option<&RafxRootSignatureGles2> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(_) => None,
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
            RafxRootSignature::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_root_signature(&self) -> Option<&RafxRootSignatureGles3> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(inner) => Some(inner),
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
            RafxRootSignature::Empty(_) => None,
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
    pub fn empty_root_signature(&self) -> Option<&RafxRootSignatureEmpty> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxRootSignature::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxRootSignature::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxRootSignature::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxRootSignature::Gles3(_) => None,
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
            RafxRootSignature::Empty(inner) => Some(inner),
        }
    }
}
