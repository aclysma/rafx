#[cfg(feature = "rafx-metal")]
use crate::metal::RafxShaderModuleMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxShaderModuleVulkan;

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
}

impl RafxShaderModule {
    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_shader_module(&self) -> Option<&RafxShaderModuleVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShaderModule::Vk(shader_module) => Some(shader_module),
            #[cfg(feature = "rafx-metal")]
            RafxShaderModule::Metal(_shader_module) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_shader_module(&self) -> Option<&RafxShaderModuleMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxShaderModule::Vk(_shader_module) => None,
            #[cfg(feature = "rafx-metal")]
            RafxShaderModule::Metal(shader_module) => Some(shader_module),
        }
    }
}
