use crate::{RafxRenderTarget, RafxTexture, RafxTextureDef};

#[derive(Debug)]
pub enum RafxImage {
    Texture(RafxTexture),
    RenderTarget(RafxRenderTarget),
}

impl RafxImage {
    pub fn texture_def(&self) -> &RafxTextureDef {
        match self {
            RafxImage::Texture(inner) => inner.texture_def(),
            RafxImage::RenderTarget(inner) => inner.texture().texture_def(),
        }
    }

    pub fn texture(&self) -> &RafxTexture {
        match self {
            RafxImage::Texture(inner) => inner,
            RafxImage::RenderTarget(inner) => inner.texture(),
        }
    }

    pub fn render_target(&self) -> Option<&RafxRenderTarget> {
        match self {
            RafxImage::Texture(_inner) => None,
            RafxImage::RenderTarget(inner) => Some(inner),
        }
    }
}

impl From<RafxTexture> for RafxImage {
    fn from(texture: RafxTexture) -> Self {
        RafxImage::Texture(texture)
    }
}

impl From<RafxRenderTarget> for RafxImage {
    fn from(render_target: RafxRenderTarget) -> Self {
        RafxImage::RenderTarget(render_target)
    }
}
