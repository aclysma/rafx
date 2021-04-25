use crate::gl::gles20::types::GLenum;
use crate::gl::{gles20, RafxDeviceContextGl, RenderbufferId, TextureId, NONE_TEXTURE};
use crate::{GlTextureFormatInfo, RafxResourceType, RafxResult, RafxTextureDef, RafxTextureDimensions, RafxSampleCount};
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub enum RafxRawImageGl {
    //Renderbuffer(RenderbufferId),
    Texture(TextureId),
}

impl RafxRawImageGl {
    pub fn gl_texture_id(&self) -> Option<TextureId> {
        match self {
            //RafxRawImageGl::Renderbuffer(_) => None,
            RafxRawImageGl::Texture(id) => Some(*id),
        }
    }

    // pub fn gl_renderbuffer_id(&self) -> Option<RenderbufferId> {
    //     match self {
    //         //RafxRawImageGl::Renderbuffer(id) => Some(*id),
    //         RafxRawImageGl::Texture(_) => None,
    //     }
    // }
}

#[derive(Debug)]
pub struct RafxTextureGlInner {
    device_context: RafxDeviceContextGl,
    texture_def: RafxTextureDef,
    image: RafxRawImageGl,
    gl_target: GLenum,
    texture_id: u32,
    format_info: GlTextureFormatInfo,
}

impl Drop for RafxTextureGlInner {
    fn drop(&mut self) {
        match self.image {
            //RafxRawImageGl::Renderbuffer(_) => {} // do nothing
            RafxRawImageGl::Texture(texture_id) => self
                .device_context
                .gl_context()
                .gl_destroy_texture(texture_id)
                .unwrap(),
        }
    }
}

/// Holds the vk::Image and allocation as well as a few vk::ImageViews depending on the
/// provided RafxResourceType in the texture_def.
#[derive(Clone, Debug)]
pub struct RafxTextureGl {
    inner: Arc<RafxTextureGlInner>,
}

impl PartialEq for RafxTextureGl {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.inner.texture_id == other.inner.texture_id
    }
}

impl Eq for RafxTextureGl {}

impl Hash for RafxTextureGl {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.inner.texture_id.hash(state);
    }
}

impl RafxTextureGl {
    pub fn texture_def(&self) -> &RafxTextureDef {
        &self.inner.texture_def
    }

    pub fn gl_raw_image(&self) -> &RafxRawImageGl {
        &self.inner.image
    }

    pub fn gl_target(&self) -> GLenum {
        self.inner.gl_target
    }

    pub fn gl_format_info(&self) -> &GlTextureFormatInfo {
        &self.inner.format_info
    }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureGl> {
        Self::from_existing(device_context, None, texture_def)
    }

    // This path is mostly so we can wrap a provided swapchain image
    pub fn from_existing(
        device_context: &RafxDeviceContextGl,
        existing_image: Option<RafxRawImageGl>,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureGl> {
        texture_def.verify();

        if texture_def.sample_count != RafxSampleCount::SampleCount1 {
            unimplemented!("GL ES 2.0 backend does not implement multisampled images");
        }

        let dimensions = texture_def
            .dimensions
            .determine_dimensions(texture_def.extents);

        if dimensions != RafxTextureDimensions::Dim2D {
            unimplemented!("GL ES 2.0 only supports 2D textures");
        }

        let gl_target = if texture_def
            .resource_type
            .contains(RafxResourceType::TEXTURE_CUBE)
        {
            log::info!("texture cube map");
            gles20::TEXTURE_CUBE_MAP
        } else {
            log::info!("texture2d");
            gles20::TEXTURE_2D
        };

        let format_info = texture_def
            .format
            .gl_texture_format_info()
            .ok_or_else(|| format!("Format {:?} not supported", texture_def.format))?;

        let image = if let Some(existing_image) = existing_image {
            existing_image
        } else {
            let gl_context = device_context.gl_context();
            let texture_id = gl_context.gl_create_texture()?;
            gl_context.gl_bind_texture(gl_target, texture_id)?;
            gl_context.gl_tex_image_2d(
                gl_target,
                0,
                format_info.gl_internal_format,
                texture_def.extents.width,
                texture_def.extents.height,
                0,
                format_info.gl_format,
                format_info.gl_type,
                None
            )?;
            gl_context.gl_bind_texture(gl_target, NONE_TEXTURE)?;

            RafxRawImageGl::Texture(texture_id)
        };

        let texture_id = crate::internal_shared::NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed);

        let inner = RafxTextureGlInner {
            device_context: device_context.clone(),
            image,
            texture_def: texture_def.clone(),
            gl_target,
            texture_id,
            format_info,
        };

        return Ok(RafxTextureGl {
            inner: Arc::new(inner),
        });
    }
}
