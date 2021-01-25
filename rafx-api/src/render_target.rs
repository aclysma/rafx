#[cfg(feature = "rafx-metal")]
use crate::metal::RafxRenderTargetMetal;
#[cfg(feature = "rafx-vulkan")]
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
    #[cfg(feature = "rafx-vulkan")]
    Vk(RafxRenderTargetVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxRenderTargetMetal),
}

impl RafxRenderTarget {
    /// Return the metadata used to create the render target
    pub fn render_target_def(&self) -> &RafxRenderTargetDef {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRenderTarget::Vk(inner) => inner.render_target_def(),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(inner) => inner.render_target_def(),
        }
    }

    /// Returns this render target as a texture. (All render targets can be used as textures.)
    pub fn texture(&self) -> &RafxTexture {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRenderTarget::Vk(inner) => inner.texture(),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(inner) => inner.texture(),
        }
    }

    /// Get the underlying vulkan API object. This provides access to any internally created
    /// vulkan objects.
    #[cfg(feature = "rafx-vulkan")]
    pub fn vk_render_target(&self) -> Option<&RafxRenderTargetVulkan> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRenderTarget::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(_inner) => None,
        }
    }

    /// Get the underlying metal API object. This provides access to any internally created
    /// metal objects.
    #[cfg(feature = "rafx-metal")]
    pub fn metal_render_target(&self) -> Option<&RafxRenderTargetMetal> {
        match self {
            #[cfg(feature = "rafx-vulkan")]
            RafxRenderTarget::Vk(_inner) => None,
            #[cfg(feature = "rafx-metal")]
            RafxRenderTarget::Metal(inner) => Some(inner),
        }
    }
}
