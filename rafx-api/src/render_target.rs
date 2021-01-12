#[cfg(feature = "rafx-metal")]
use crate::metal::RafxRenderTargetMetal;
use crate::vulkan::RafxRenderTargetVulkan;
use crate::{RafxRenderTargetDef, RafxTexture};

// This is clone because the swapchain provides images with other resources (like vulkan image
// views) and it's better to share those than duplicate. As long as I'm having to Arc them, might
// as well expose cloneable
#[derive(Clone, Debug)]
pub enum RafxRenderTarget {
    Vk(RafxRenderTargetVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxRenderTargetMetal),
}

impl RafxRenderTarget {
    pub fn render_target_def(&self) -> &RafxRenderTargetDef {
        match self {
            RafxRenderTarget::Vk(inner) => inner.render_target_def(),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn texture(&self) -> &RafxTexture {
        match self {
            RafxRenderTarget::Vk(inner) => inner.texture(),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(_inner) => unimplemented!(),
        }
    }

    // each render target gets a new ID, meant for hashing
    pub(crate) fn render_target_id(&self) -> u32 {
        match self {
            RafxRenderTarget::Vk(inner) => inner.render_target_id(),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(_inner) => unimplemented!(),
        }
    }
    pub(crate) fn take_is_undefined_layout(&self) -> bool {
        match self {
            RafxRenderTarget::Vk(inner) => inner.take_is_undefined_layout(),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_render_target(&self) -> Option<&RafxRenderTargetVulkan> {
        match self {
            RafxRenderTarget::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_render_target(&self) -> Option<&RafxRenderTargetMetal> {
        match self {
            RafxRenderTarget::Vk(_inner) => None,
            RafxRenderTarget::Metal(inner) => Some(inner),
        }
    }
}
