#[cfg(any(
    feature = "rafx-empty",
    not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
))]
use crate::empty::RafxTextureEmpty;
#[cfg(feature = "rafx-gl")]
use crate::gl::RafxTextureGl;
#[cfg(feature = "rafx-metal")]
use crate::metal::RafxTextureMetal;
#[cfg(feature = "rafx-vulkan")]
use crate::vulkan::RafxTextureVulkan;
use crate::RafxTextureDef;

/// An image that can be used by the GPU.
///
/// Textures must not be dropped if they are in use by the GPU.
#[derive(Clone, Debug)]
pub enum RafxTexture {
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxTextureVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxTextureMetal),
    #[cfg(feature = "rafx-gl")]
    Gl(RafxTextureGl),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
    ))]
    Empty(RafxTextureEmpty),
}

impl RafxTexture {
    /// Return the metadata used to create the texture
    pub fn texture_def(&self) -> &RafxTextureDef {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(inner) => inner.texture_def(),
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(inner) => inner.texture_def(),
            #[cfg(feature = "rafx-gl")]
            RafxTexture::Gl(inner) => inner.texture_def(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxTexture::Empty(inner) => inner.texture_def(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_texture(&self) -> Option<&RafxTextureVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_) => None,
            #[cfg(feature = "rafx-gl")]
            RafxTexture::Gl(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxTexture::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_texture(&self) -> Option<&RafxTextureMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(inner) => Some(inner),
            #[cfg(feature = "rafx-gl")]
            RafxTexture::Gl(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxTexture::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gl")]
    pub fn gl_texture(&self) -> Option<&RafxTextureGl> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_) => None,
            #[cfg(feature = "rafx-gl")]
            RafxTexture::Gl(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxTexture::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
    ))]
    pub fn empty_texture(&self) -> Option<&RafxTextureEmpty> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_) => None,
            #[cfg(feature = "rafx-gl")]
            RafxTexture::Gl(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(feature = "rafx-metal", feature = "rafx-vulkan"))
            ))]
            RafxTexture::Empty(inner) => Some(inner),
        }
    }
}
