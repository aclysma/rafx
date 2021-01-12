#[cfg(feature = "rafx-metal")]
use crate::metal::RafxShaderModuleMetal;
use crate::vulkan::RafxShaderModuleVulkan;

/// Shader modules represent loaded shader code. Different APIs require different forms of input.
/// A shader module represents that input having completed the initial "loading" process and is
/// ready to be used to create pipelines
#[derive(Clone, Debug)]
pub enum RafxShaderModule {
    Vk(RafxShaderModuleVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxShaderModuleMetal),
}

impl RafxShaderModule {
    pub fn vk_shader_module(&self) -> Option<&RafxShaderModuleVulkan> {
        match self {
            RafxShaderModule::Vk(shader_module) => Some(shader_module),
            #[cfg(feature = "rafx-metal")]
            RafxShaderModule::Metal(_shader_module) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_shader_module(&self) -> Option<&RafxShaderModuleMetal> {
        match self {
            RafxShaderModule::Vk(_shader_module) => None,
            RafxShaderModule::Metal(shader_module) => Some(shader_module),
        }
    }
}
