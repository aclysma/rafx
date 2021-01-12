#[cfg(feature = "rafx-metal")]
use crate::metal::RafxShaderMetal;
use crate::vulkan::RafxShaderVulkan;

#[derive(Clone, Debug)]
pub enum RafxShader {
    Vk(RafxShaderVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxShaderMetal),
}

impl RafxShader {
    pub fn vk_shader(&self) -> Option<&RafxShaderVulkan> {
        match self {
            RafxShader::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxShader::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_shader(&self) -> Option<&RafxShaderMetal> {
        match self {
            RafxShader::Vk(_inner) => None,
            RafxShader::Metal(inner) => Some(inner),
        }
    }
}
