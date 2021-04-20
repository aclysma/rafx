use crate::gl::{RafxDeviceContextGl, RenderbufferId, TextureId, NONE_RENDERBUFFER};
use crate::{
    RafxMemoryUsage, RafxResourceType, RafxResult, RafxSampleCount, RafxTextureDef,
    RafxTextureDimensions,
};
use std::hash::{Hash, Hasher};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::process::exit;

#[derive(Debug, PartialEq)]
pub enum RafxRawImageGl {
    Renderbuffer(RenderbufferId),
    Texture(TextureId)
}

impl RafxRawImageGl {
    pub fn gl_texture_id(&self) -> Option<TextureId> {
        match self {
            RafxRawImageGl::Renderbuffer(_) => None,
            RafxRawImageGl::Texture(id) => Some(*id)
        }
    }

    pub fn gl_renderbuffer_id(&self) -> Option<RenderbufferId> {
        match self {
            RafxRawImageGl::Renderbuffer(id) => Some(*id),
            RafxRawImageGl::Texture(_) => None
        }
    }
}

#[derive(Debug)]
pub struct RafxTextureGlInner {
    device_context: RafxDeviceContextGl,
    texture_def: RafxTextureDef,
    image: RafxRawImageGl,
    //mip_level_uav_views: Vec<gl_rs::Texture>,
    texture_id: u32,
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

    // pub fn gl_texture(&self) -> &gl_rs::TextureRef {
    //     self.inner.image.gl_texture()
    // }
    //
    // pub fn gl_mip_level_uav_views(&self) -> &[gl_rs::Texture] {
    //     &self.inner.mip_level_uav_views
    // }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureGl> {
        unimplemented!();
        Self::from_existing(device_context, None, texture_def)
    }

    // This path is mostly so we can wrap a provided swapchain image
    pub fn from_existing(
        device_context: &RafxDeviceContextGl,
        existing_image: Option<RafxRawImageGl>,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureGl> {
        texture_def.verify();

        if let Some(existing_image) = existing_image {
            if existing_image == RafxRawImageGl::Renderbuffer(NONE_RENDERBUFFER) {
                let texture_id = crate::internal_shared::NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed);

                let inner = RafxTextureGlInner {
                    device_context: device_context.clone(),
                    image: existing_image,
                    texture_def: texture_def.clone(),
                    texture_id
                };

                return Ok(RafxTextureGl {
                    inner: Arc::new(inner)
                })
            }
        }

        unimplemented!();



        // unimplemented!();

        // let dimensions = texture_def
        //     .dimensions
        //     .determine_dimensions(texture_def.extents);
        //
        // let (mtl_texture_type, mtl_array_length) = match dimensions {
        //     RafxTextureDimensions::Dim1D => {
        //         if texture_def.array_length > 1 {
        //             if !device_context.gl_features().supports_array_of_textures {
        //                 return Err("Texture arrays not supported")?;
        //             }
        //
        //             (MTLTextureType::D1Array, texture_def.array_length)
        //         } else {
        //             (MTLTextureType::D1, 1)
        //         }
        //     }
        //     RafxTextureDimensions::Dim2D => {
        //         if texture_def
        //             .resource_type
        //             .contains(RafxResourceType::TEXTURE_CUBE)
        //         {
        //             if texture_def.array_length <= 6 {
        //                 (MTLTextureType::Cube, 1)
        //             } else {
        //                 if !device_context
        //                     .gl_features()
        //                     .supports_cube_map_texture_arrays
        //                 {
        //                     return Err("Cube map texture arrays not supported")?;
        //                 }
        //
        //                 (MTLTextureType::CubeArray, texture_def.array_length / 6)
        //             }
        //         } else if texture_def.array_length > 1 {
        //             if !device_context.gl_features().supports_array_of_textures {
        //                 return Err("Texture arrays not supported")?;
        //             }
        //
        //             (MTLTextureType::D2Array, texture_def.array_length)
        //         } else if texture_def.sample_count != RafxSampleCount::SampleCount1 {
        //             (MTLTextureType::D2Multisample, 1)
        //         } else {
        //             (MTLTextureType::D2, 1)
        //         }
        //     }
        //     RafxTextureDimensions::Dim3D => (MTLTextureType::D3, texture_def.array_length.max(1)),
        //     _ => unreachable!(),
        // };
        //
        // let image = if let Some(existing_image) = existing_image {
        //     existing_image
        // } else {
        //     let descriptor = gl_rs::TextureDescriptor::new();
        //     descriptor.set_pixel_format(texture_def.format.into());
        //     descriptor.set_width(texture_def.extents.width as _);
        //     descriptor.set_height(texture_def.extents.height as _);
        //     descriptor.set_depth(texture_def.extents.depth as _);
        //     descriptor.set_mipmap_level_count(texture_def.mip_count as _);
        //     descriptor.set_storage_mode(RafxMemoryUsage::GpuOnly.mtl_storage_mode());
        //     descriptor.set_cpu_cache_mode(RafxMemoryUsage::GpuOnly.mtl_cpu_cache_mode());
        //     descriptor.set_resource_options(RafxMemoryUsage::GpuOnly.mtl_resource_options());
        //     descriptor.set_texture_type(mtl_texture_type);
        //     descriptor.set_array_length(mtl_array_length as _);
        //     descriptor.set_sample_count(texture_def.sample_count.into());
        //
        //     let mut mtl_usage = MTLTextureUsage::empty();
        //
        //     if texture_def
        //         .resource_type
        //         .intersects(RafxResourceType::TEXTURE)
        //     {
        //         mtl_usage |= MTLTextureUsage::ShaderRead;
        //     }
        //
        //     if texture_def.resource_type.intersects(
        //         RafxResourceType::RENDER_TARGET_DEPTH_STENCIL
        //             | RafxResourceType::RENDER_TARGET_COLOR,
        //     ) {
        //         mtl_usage |= MTLTextureUsage::RenderTarget;
        //     }
        //
        //     if texture_def
        //         .resource_type
        //         .intersects(RafxResourceType::TEXTURE_READ_WRITE)
        //     {
        //         mtl_usage |= MTLTextureUsage::PixelFormatView;
        //         mtl_usage |= MTLTextureUsage::ShaderWrite;
        //     }
        //
        //     descriptor.set_usage(mtl_usage);
        //
        //     let texture = device_context.device().new_texture(descriptor.as_ref());
        //     RafxRawImageGl::Owned(texture)
        // };
        //
        // let mut mip_level_uav_views = vec![];
        // if texture_def
        //     .resource_type
        //     .intersects(RafxResourceType::TEXTURE_READ_WRITE)
        // {
        //     let uav_texture_type = match mtl_texture_type {
        //         MTLTextureType::Cube => MTLTextureType::D2Array,
        //         MTLTextureType::CubeArray => MTLTextureType::D2Array,
        //         _ => mtl_texture_type,
        //     };
        //
        //     let slices = gl_rs::NSRange::new(0, mtl_array_length as _);
        //     for mip_level in 0..texture_def.mip_count {
        //         let levels = gl_rs::NSRange::new(mip_level as _, 1);
        //         mip_level_uav_views.push(image.gl_texture().new_texture_view_from_slice(
        //             texture_def.format.into(),
        //             uav_texture_type,
        //             levels,
        //             slices,
        //         ));
        //     }
        // }
        //
        // let texture_id = crate::internal_shared::NEXT_TEXTURE_ID.fetch_add(1, Ordering::Relaxed);
        //
        // let inner = RafxTextureGlInner {
        //     texture_def: texture_def.clone(),
        //     device_context: device_context.clone(),
        //     image,
        //     mip_level_uav_views,
        //     texture_id,
        // };
        //
        // Ok(RafxTextureGl {
        //     inner: Arc::new(inner),
        // })
    }
}
