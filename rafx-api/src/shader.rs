#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2"
    ))
))]
use crate::empty::RafxShaderEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxShaderGles2;
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
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxShaderGles2),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2"
        ))
    ))]
    Empty(RafxShaderEmpty),
}

impl RafxShader {
    pub fn pipeline_reflection(&self) -> &RafxPipelineReflection {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShader::Vk(inner) => inner.pipeline_reflection(),
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(inner) => inner.pipeline_reflection(),
            #[cfg(feature = "rafx-gles2")]
            RafxShader::Gles2(inner) => inner.pipeline_reflection(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxShader::Empty(inner) => inner.pipeline_reflection(),
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
            RafxShader::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxShader::Gles2(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxShader::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_shader(&self) -> Option<&RafxShaderMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShader::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gles2")]
            RafxShader::Gles2(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxShader::Empty(_) => None,
        }
    }

    /// Get the underlying gl API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_shader(&self) -> Option<&RafxShaderGles2> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShader::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxShader::Gles2(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxShader::Empty(_) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn empty_shader(&self) -> Option<&RafxShaderMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShader::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxShader::Gles2(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2"
                ))
            ))]
            RafxShader::Empty(inner) => Some(inner),
        }
    }
}
