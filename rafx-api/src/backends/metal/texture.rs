use crate::metal::RafxDeviceContextMetal;
use crate::{
    RafxMemoryUsage, RafxResourceType, RafxResult, RafxSampleCount, RafxTextureDef,
    RafxTextureDimensions,
};
use metal_rs::{MTLTextureType, MTLTextureUsage};

#[derive(Debug)]
pub enum RafxRawImageMetal {
    Owned(metal_rs::Texture),
    Ref(metal_rs::Texture),
    //Null,
}

// for metal_rs::Texture
unsafe impl Send for RafxRawImageMetal {}
unsafe impl Sync for RafxRawImageMetal {}

impl RafxRawImageMetal {
    pub fn metal_texture(&self) -> &metal_rs::TextureRef {
        match self {
            RafxRawImageMetal::Owned(owned) => owned.as_ref(),
            RafxRawImageMetal::Ref(r) => r.as_ref(),
            //RafxRawImageMetal::Null => None
        }
    }
}

/// Holds the vk::Image and allocation as well as a few vk::ImageViews depending on the
/// provided RafxResourceType in the texture_def.
#[derive(Debug)]
pub struct RafxTextureMetal {
    device_context: RafxDeviceContextMetal,
    texture_def: RafxTextureDef,
    image: RafxRawImageMetal,
    mip_level_uav_views: Vec<metal_rs::Texture>,
}

// for metal_rs::Texture
unsafe impl Send for RafxTextureMetal {}
unsafe impl Sync for RafxTextureMetal {}

impl RafxTextureMetal {
    pub fn texture_def(&self) -> &RafxTextureDef {
        &self.texture_def
    }

    pub fn metal_texture(&self) -> &metal_rs::TextureRef {
        self.image.metal_texture()
    }

    pub fn metal_mip_level_uav_views(&self) -> &[metal_rs::Texture] {
        &self.mip_level_uav_views
    }

    pub fn new(
        device_context: &RafxDeviceContextMetal,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureMetal> {
        Self::from_existing(device_context, None, texture_def)
    }

    // This path is mostly so we can wrap a provided swapchain image
    pub fn from_existing(
        device_context: &RafxDeviceContextMetal,
        existing_image: Option<RafxRawImageMetal>,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureMetal> {
        texture_def.verify();

        let dimensions = texture_def
            .dimensions
            .determine_dimensions(texture_def.extents);

        let (mtl_texture_type, mtl_array_length) = match dimensions {
            RafxTextureDimensions::Dim1D => {
                if texture_def.array_length > 1 {
                    if !device_context.metal_features().supports_array_of_textures {
                        return Err("Texture arrays not supported")?;
                    }

                    (MTLTextureType::D1Array, texture_def.array_length)
                } else {
                    (MTLTextureType::D1, 1)
                }
            }
            RafxTextureDimensions::Dim2D => {
                if texture_def
                    .resource_type
                    .contains(RafxResourceType::TEXTURE_CUBE)
                {
                    if texture_def.array_length <= 6 {
                        (MTLTextureType::Cube, 1)
                    } else {
                        if !device_context
                            .metal_features()
                            .supports_cube_map_texture_arrays
                        {
                            return Err("Cube map texture arrays not supported")?;
                        }

                        (MTLTextureType::CubeArray, texture_def.array_length / 6)
                    }
                } else if texture_def.array_length > 1 {
                    if !device_context.metal_features().supports_array_of_textures {
                        return Err("Texture arrays not supported")?;
                    }

                    (MTLTextureType::D2Array, texture_def.array_length)
                } else if texture_def.sample_count != RafxSampleCount::SampleCount1 {
                    (MTLTextureType::D2Multisample, 1)
                } else {
                    (MTLTextureType::D2, 1)
                }
            }
            RafxTextureDimensions::Dim3D => (MTLTextureType::D3, texture_def.array_length.max(1)),
            _ => unreachable!(),
        };

        let image = if let Some(existing_image) = existing_image {
            existing_image
        } else {
            let descriptor = metal_rs::TextureDescriptor::new();
            descriptor.set_pixel_format(texture_def.format.into());
            descriptor.set_width(texture_def.extents.width as _);
            descriptor.set_height(texture_def.extents.height as _);
            descriptor.set_depth(texture_def.extents.depth as _);
            descriptor.set_mipmap_level_count(texture_def.mip_count as _);
            descriptor.set_storage_mode(RafxMemoryUsage::GpuOnly.mtl_storage_mode());
            descriptor.set_cpu_cache_mode(RafxMemoryUsage::GpuOnly.mtl_cpu_cache_mode());
            descriptor.set_resource_options(RafxMemoryUsage::GpuOnly.mtl_resource_options());
            descriptor.set_texture_type(mtl_texture_type);
            descriptor.set_array_length(mtl_array_length as _);
            descriptor.set_sample_count(texture_def.sample_count.into());

            let mut mtl_usage = MTLTextureUsage::empty();

            if texture_def
                .resource_type
                .intersects(RafxResourceType::TEXTURE)
            {
                mtl_usage |= MTLTextureUsage::ShaderRead;
            }

            if texture_def.resource_type.intersects(
                RafxResourceType::RENDER_TARGET_DEPTH_STENCIL
                    | RafxResourceType::RENDER_TARGET_COLOR,
            ) {
                mtl_usage |= MTLTextureUsage::RenderTarget;
            }

            if texture_def
                .resource_type
                .intersects(RafxResourceType::TEXTURE_READ_WRITE)
            {
                mtl_usage |= MTLTextureUsage::PixelFormatView;
                mtl_usage |= MTLTextureUsage::ShaderWrite;
            }

            descriptor.set_usage(mtl_usage);

            let texture = device_context.device().new_texture(descriptor.as_ref());
            RafxRawImageMetal::Owned(texture)
        };

        let mut mip_level_uav_views = vec![];
        if texture_def
            .resource_type
            .intersects(RafxResourceType::TEXTURE_READ_WRITE)
        {
            let uav_texture_type = match mtl_texture_type {
                MTLTextureType::Cube => MTLTextureType::D2Array,
                MTLTextureType::CubeArray => MTLTextureType::D2Array,
                _ => mtl_texture_type,
            };

            let slices = metal_rs::NSRange::new(0, mtl_array_length as _);
            for mip_level in 0..texture_def.mip_count {
                let levels = metal_rs::NSRange::new(mip_level as _, 1);
                mip_level_uav_views.push(image.metal_texture().new_texture_view_from_slice(
                    texture_def.format.into(),
                    uav_texture_type,
                    levels,
                    slices,
                ));
            }
        }

        Ok(RafxTextureMetal {
            texture_def: texture_def.clone(),
            device_context: device_context.clone(),
            image,
            mip_level_uav_views,
        })
    }
}
