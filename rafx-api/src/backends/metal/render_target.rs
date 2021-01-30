use crate::metal::{RafxDeviceContextMetal, RafxRawImageMetal, RafxTextureMetal};
use crate::{RafxRenderTargetDef, RafxResult, RafxTexture};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static RENDER_TARGET_NEXT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug)]
pub struct RafxRenderTargetMetalInner {
    // It's a RafxTextureMetal, but stored as RafxTexture so that we can return refs to it
    pub texture: RafxTexture,
    //is_undefined_layout: AtomicBool,
    pub render_target_def: RafxRenderTargetDef,
    render_target_id: u32,
}

#[derive(Clone, Debug)]
pub struct RafxRenderTargetMetal {
    inner: Arc<RafxRenderTargetMetalInner>,
}

impl PartialEq for RafxRenderTargetMetal {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.inner.render_target_id == other.inner.render_target_id
    }
}

impl Eq for RafxRenderTargetMetal {}

impl Hash for RafxRenderTargetMetal {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.inner.render_target_id.hash(state);
    }
}

impl RafxRenderTargetMetal {
    pub fn render_target_def(&self) -> &RafxRenderTargetDef {
        &self.inner.render_target_def
    }

    pub fn texture(&self) -> &RafxTexture {
        &self.inner.texture
    }

    pub fn new(
        device_context: &RafxDeviceContextMetal,
        render_target_def: &RafxRenderTargetDef,
    ) -> RafxResult<Self> {
        Self::from_existing(device_context, None, render_target_def)
    }

    pub fn from_existing(
        device_context: &RafxDeviceContextMetal,
        existing_image: Option<RafxRawImageMetal>,
        render_target_def: &RafxRenderTargetDef,
    ) -> RafxResult<Self> {
        render_target_def.verify();

        let texture_def = render_target_def.to_texture_def();

        let texture =
            RafxTextureMetal::from_existing(device_context, existing_image, &texture_def)?;

        let render_target_id = RENDER_TARGET_NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let inner = RafxRenderTargetMetalInner {
            texture: RafxTexture::Metal(texture),
            //is_undefined_layout: AtomicBool::new(true),
            render_target_def: render_target_def.clone(),
            render_target_id,
        };

        Ok(RafxRenderTargetMetal {
            inner: Arc::new(inner),
        })
    }
}
