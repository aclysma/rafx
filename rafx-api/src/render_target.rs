#[cfg(feature = "rafx-metal")]
use crate::metal::RafxRenderTargetMetal;
use crate::vulkan::RafxRenderTargetVulkan;
use crate::{RafxRenderTargetDef, RafxTexture};

/// Render targets are writable textures that the GPU can render to.
///
/// The general flow is to bind a render target and pipeline to a command buffer and draw
/// primitives.
///
/// Render targets can generally be used in APIs that accept textures. (However they may require
/// being transitioned to a state appropriate to how they will be used.)
///
/// Render targets are cloneable because they are sometimes owned by the swapchain. Using a
/// render target provided by a swapchain after the swapchain has been destroyed will result in
/// undefined behavior. However, a render target created by an application can be used as long as
/// it is not dropped and as long as the GPU is using it.
///
/// Render targets not owned by a swapchain must not be dropped if they are in use by the GPU.
/// (Individual clones may be dropped, but one of the instances must remain)
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
