#[cfg(any(
    feature = "rafx-empty",
    not(any(
        feature = "rafx-metal",
        feature = "rafx-vulkan",
        feature = "rafx-gles2",
        feature = "rafx-gles3"
    ))
))]
use crate::empty::RafxTextureEmpty;
#[cfg(feature = "rafx-gles2")]
use crate::gles2::RafxTextureGles2;
#[cfg(feature = "rafx-gles3")]
use crate::gles3::RafxTextureGles3;
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
    #[cfg(feature = "rafx-gles2")]
    Gles2(RafxTextureGles2),
    #[cfg(feature = "rafx-gles3")]
    Gles3(RafxTextureGles3),
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
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
            #[cfg(feature = "rafx-gles2")]
            RafxTexture::Gles2(inner) => inner.texture_def(),
            #[cfg(feature = "rafx-gles3")]
            RafxTexture::Gles3(inner) => inner.texture_def(),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxTexture::Empty(inner) => inner.texture_def(),
        }
    }

    /// Sets a name for this texture. This is useful for debugging, graphics debuggers/profilers such
    /// as nsight graphics or renderdoc will display this texture with the given name in the list of resources.
    pub fn set_debug_name(
        &self,
        _name: impl AsRef<str>,
    ) {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(inner) => inner.set_debug_name(_name),
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(inner) => inner.set_debug_name(_name),
            #[cfg(feature = "rafx-gles2")]
            RafxTexture::Gles2(_) => {}
            #[cfg(feature = "rafx-gles3")]
            RafxTexture::Gles3(_) => {}
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxTexture::Empty(inner) => inner.set_debug_name(_name),
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
            #[cfg(feature = "rafx-gles2")]
            RafxTexture::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxTexture::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
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
            #[cfg(feature = "rafx-gles2")]
            RafxTexture::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxTexture::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxTexture::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles2")]
    pub fn gles2_texture(&self) -> Option<&RafxTextureGles2> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxTexture::Gles2(inner) => Some(inner),
            #[cfg(feature = "rafx-gles3")]
            RafxTexture::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxTexture::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-gles3")]
    pub fn gles3_texture(&self) -> Option<&RafxTextureGles3> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxTexture::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxTexture::Gles3(inner) => Some(inner),
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxTexture::Empty(_) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(any(
        feature = "rafx-empty",
        not(any(
            feature = "rafx-metal",
            feature = "rafx-vulkan",
            feature = "rafx-gles2",
            feature = "rafx-gles3"
        ))
    ))]
    pub fn empty_texture(&self) -> Option<&RafxTextureEmpty> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxTexture::Vk(_) => None,
            #[cfg(feature = "rafx-metal")]
            RafxTexture::Metal(_) => None,
            #[cfg(feature = "rafx-gles2")]
            RafxTexture::Gles2(_) => None,
            #[cfg(feature = "rafx-gles3")]
            RafxTexture::Gles3(_) => None,
            #[cfg(any(
                feature = "rafx-empty",
                not(any(
                    feature = "rafx-metal",
                    feature = "rafx-vulkan",
                    feature = "rafx-gles2",
                    feature = "rafx-gles3"
                ))
            ))]
            RafxTexture::Empty(inner) => Some(inner),
        }
    }
}
