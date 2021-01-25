#[cfg(feature = "rafx-metal")]
use crate::metal::RafxShaderMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxShaderVulkan;
use crate::RafxPipelineReflection;

/// Represents one or more shader stages, producing an entire "program" to execute on the GPU
#[derive(Clone, Debug)]
pub enum RafxShader {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxShaderVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxShaderMetal),
}

impl RafxShader {
    pub fn pipeline_reflection(&self) -> &RafxPipelineReflection {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShader::Vk(inner) => inner.pipeline_reflection(),
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(inner) => inner.pipeline_reflection(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_shader(&self) -> Option<&RafxShaderVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShader::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_shader(&self) -> Option<&RafxShaderMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShader::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(inner) => Some(inner),
        }
    }
}
