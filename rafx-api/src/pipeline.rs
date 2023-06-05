#[cfg(feature = "rafx-dx12")]
use crate::dx12::RafxPipelineDx12;
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
use crate::empty::RafxPipelineEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxPipelineGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxPipelineGles3;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxPipelineMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxPipelineVulkan;
use crate::{RafxPipelineType, RafxRootSignature};

/// Represents a complete GPU configuration for executing work.
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
    #[cfg(feature = "rafx-dx12")]
    Dx12(RafxPipelineDx12),
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxPipelineVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxPipelineMetal),
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxPipelineGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxPipelineGles3),
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
    Empty(RafxPipelineEmpty),
}

impl RafxPipeline {
    /// Returns the type of pipeline that this is
    pub fn pipeline_type(&self) -> RafxPipelineType {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(inner) => inner.pipeline_type(),
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(inner) => inner.pipeline_type(),
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
            RafxPipeline::Empty(inner) => inner.pipeline_type(),
        }
    }

    /// Returns the root signature used to create the pipeline
    pub fn root_signature(&self) -> &RafxRootSignature {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(inner) => inner.root_signature(),
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(inner) => inner.root_signature(),
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
            RafxPipeline::Empty(inner) => inner.root_signature(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-dx12")]
    pub fn dx12_pipeline(&self) -> Option<&RafxPipelineDx12> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(inner) => Some(inner),
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(_) => None,
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
            RafxPipeline::Empty(_) => None,
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_pipeline(&self) -> Option<&RafxPipelineVulkan> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(_) => None,
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
            RafxPipeline::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_pipeline(&self) -> Option<&RafxPipelineMetal> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(_) => None,
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
            RafxPipeline::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_pipeline(&self) -> Option<&RafxPipelineGles2> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(_) => None,
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
            RafxPipeline::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_pipeline(&self) -> Option<&RafxPipelineGles3> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(inner) => Some(inner),
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
            RafxPipeline::Empty(_) => None,
        }
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
    pub fn empty_pipeline(&self) -> Option<&RafxPipelineEmpty> {
        match self {
            #[cfg(feature = "rafx-dx12")]
            RafxPipeline::Dx12(_) => None,
            #[cfg(feature = "rafx-vulkan")]
            RafxPipeline::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxPipeline::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxPipeline::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxPipeline::Gles3(_) => None,
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
            RafxPipeline::Empty(inner) => Some(inner),
        }
    }
}
