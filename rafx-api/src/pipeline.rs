#[cfg(feature = "rafx-metal")]
use crate::metal::RafxPipelineMetal;
use crate::vulkan::RafxPipelineVulkan;
use crate::{RafxPipelineType, RafxRootSignature};

/// Represents a complete GPU configuration executing work.
///
/// There are two kinds of pipelines: Graphics and Compute
///
/// A pipeline includes fixed-function state (i.e. configuration) and programmable state
/// (i.e. shaders). Pipelines are expensive objects to create. Ideally, they should be created
/// when the application initializes or on a separate thread.
///
/// Pipelines are bound by command buffers. Fewer pipeline changes is better, and it is often worth
/// batching draw calls that use the same pipeline to happen together so that the pipeline does not
/// need to be changed as frequently.
///
/// Pipelines must not be dropped if they are in use by the GPU.
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
