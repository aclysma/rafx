#[cfg(feature = "rafx-metal")]
use crate::metal::RafxTextureMetal;
use crate::vulkan::RafxTextureVulkan;
use crate::RafxTextureDef;

/// An image that can be used by the GPU.
///
/// Textures must not be dropped if they are in use by the GPU.
#[derive(Debug)]
pub enum RafxTexture {
    Vk(RafxTextureVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxTextureMetal),
}

impl RafxTexture {
    pub fn texture_def(&self) -> &RafxTextureDef {
        match self {
            RafxTexture::Vk(inner) => inner.texture_def(),
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_texture(&self) -> Option<&RafxTextureVulkan> {
        match self {
            RafxTexture::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_texture(&self) -> Option<&RafxTextureMetal> {
        match self {
            RafxTexture::Vk(_inner) => None,
            RafxTexture::Metal(inner) => Some(inner),
        }
    }
}
