#[cfg(feature = "rafx-metal")]
use crate::metal::RafxPipelineMetal;
#[cfg(feature = "rafx-vulkan")]
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
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxPipelineVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxPipelineMetal),
}

impl RafxPipeline {
    /// Returns the type of pipeline that this is
    pub fn pipeline_type(&self) -> RafxPipelineType {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(inner) => inner.pipeline_type(),
        }
    }

    /// Returns the root signature used to create the pipeline
    pub fn root_signature(&self) -> &RafxRootSignature {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(inner) => inner.root_signature(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_pipeline(&self) -> Option<&RafxPipelineVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_pipeline(&self) -> Option<&RafxPipelineMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(inner) => Some(inner),
        }
    }
}
