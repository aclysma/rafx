#[cfg(feature = "rafx-metal")]
use crate::metal::RafxPipelineMetal;
use crate::vulkan::RafxPipelineVulkan;
use crate::{RafxPipelineType, RafxRootSignature};

#[derive(Debug)]
pub enum RafxPipeline {
    Vk(RafxPipelineVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxPipelineMetal),
}

impl RafxPipeline {
    pub fn pipeline_type(&self) -> RafxPipelineType {
        match self {
            RafxPipeline::Vk(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn root_signature(&self) -> &RafxRootSignature {
        match self {
            RafxPipeline::Vk(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_pipeline(&self) -> Option<&RafxPipelineVulkan> {
        match self {
            RafxPipeline::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_pipeline(&self) -> Option<&RafxPipelineMetal> {
        match self {
            RafxPipeline::Vk(_inner) => None,
            RafxPipeline::Metal(inner) => Some(inner),
        }
    }
}
