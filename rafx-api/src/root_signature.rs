#[cfg(feature = "rafx-metal")]
use crate::metal::RafxRootSignatureMetal;
use crate::vulkan::RafxRootSignatureVulkan;

#[derive(Clone, Debug)]
pub enum RafxRootSignature {
    Vk(RafxRootSignatureVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxRootSignatureMetal),
}

impl RafxRootSignature {
    pub fn vk_root_signature(&self) -> Option<&RafxRootSignatureVulkan> {
        match self {
            RafxRootSignature::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxRootSignature::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_root_signature(&self) -> Option<&RafxRootSignatureMetal> {
        match self {
            RafxRootSignature::Vk(_inner) => None,
            RafxRootSignature::Metal(inner) => Some(inner),
        }
    }
}
