#[cfg(any(
    feature = "rafx-empty",
    not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
))]
use crate::empty::RafxShaderModuleEmpty;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxShaderModuleMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxShaderModuleVulkan;
#[cfg(feature = "rafx-gl")]
use crate::gl::RafxShaderModuleGl;

/// Rrepresents loaded shader code that can be used to create a pipeline.
///
/// Different APIs require different forms of input. A shader module is created by a "loading"
/// process that is API-specific. This form could be compiled binary or uncompiled shader code,
/// depending on the backend in use.
#[derive(Clone, Debug)]
pub enum RafxShaderModule {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxShaderModuleVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxShaderModuleMetal),
    #[cfg(feature = "rafx-gl")]
    Gl(RafxShaderModuleGl),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
    ))]
    Empty(RafxShaderModuleEmpty),
}

impl RafxShaderModule {
    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_shader_module(&self) -> Option<&RafxShaderModuleVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShaderModule::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxShaderModule::Metal(_) => None,
            #[cfg(feature = "rafx-gl")]
            RafxShaderModule::Gl(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxShaderModule::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_shader_module(&self) -> Option<&RafxShaderModuleMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShaderModule::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShaderModule::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gl")]
            RafxShaderModule::Gl(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxShaderModule::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gl")]
    pub fn gl_shader_module(&self) -> Option<&RafxShaderModuleGl> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShaderModule::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShaderModule::Metal(_) => None,
            #[cfg(feature = "rafx-gl")]
            RafxShaderModule::Gl(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxShaderModule::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
    ))]
    pub fn empty_shader_module(&self) -> Option<&RafxShaderModuleEmpty> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShaderModule::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShaderModule::Metal(_) => None,
            #[cfg(feature = "rafx-gl")]
            RafxShaderModule::Gl(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxShaderModule::Empty(inner) => Some(inner),
        }
    }
}
