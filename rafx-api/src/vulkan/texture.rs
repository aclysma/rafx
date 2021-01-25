use crate::vulkan::RafxDeviceContextVulkan;
use crate::RafxTextureDimensions;
use crate::{RafxExtents3D, RafxResourceType, RafxResult, RafxTextureDef};
use ash::version::DeviceV1_0;
use ash::vk;

// This is used to allow the underlying image/allocation to be removed from a RafxTextureVulkan,
// or to init a RafxTextureVulkan with an existing image/allocation. If the allocation is none, we
// will not destroy the image when RafxRawImageVulkan is dropped
#[derive(Debug)]
pub struct RafxRawImageVulkan {
    pub image: vk::Image,
    pub allocation: Option<vk_mem::Allocation>,
}

impl RafxRawImageVulkan {
    fn destroy_image(
        &mut self,
        device_context: &RafxDeviceContextVulkan,
    ) {
        if let Some(allocation) = self.allocation.take() {
            log::trace!("destroying RafxImageVulkan");
            assert_ne!(self.image, vk::Image::null());
            device_context
                .allocator()
                .destroy_image(self.image, &allocation)
                .unwrap();
            self.image = vk::Image::null();
            log::trace!("destroyed RafxImageVulkan");
        } else {
            log::trace!(
                "RafxImageVulkan has no allocation associated with it, not destroying image"
            );
            self.image = vk::Image::null();
        }
    }
}

impl Drop for RafxRawImageVulkan {
    fn drop(&mut self) {
        assert!(self.allocation.is_none())
    }
}

/// Holds the vk::Image and allocation as well as a few vk::ImageViews depending on the
/// provided RafxResourceType in the texture_def.
#[derive(Debug)]
pub struct RafxTextureVulkan {
    device_context: RafxDeviceContextVulkan,
    texture_def: RafxTextureDef,
    image: RafxRawImageVulkan,
    aspect_mask: vk::ImageAspectFlags,

    // For reading
    srv_view: Option<vk::ImageView>,
    srv_view_stencil: Option<vk::ImageView>,

    // For writing
    uav_views: Vec<vk::ImageView>,
}

impl Drop for RafxTextureVulkan {
    fn drop(&mut self) {
        let device = self.device_context.device();

        unsafe {
            if let Some(srv_view) = self.srv_view {
                device.destroy_image_view(srv_view, None);
            }

            if let Some(srv_view_stencil) = self.srv_view_stencil {
                device.destroy_image_view(srv_view_stencil, None);
            }

            for uav_view in &self.uav_views {
                device.destroy_image_view(*uav_view, None);
            }
        }

        self.image.destroy_image(&self.device_context().clone());
    }
}

impl RafxTextureVulkan {
    pub fn texture_def(&self) -> &RafxTextureDef {
        &self.texture_def
    }

    pub fn extents(&self) -> &RafxExtents3D {
        &self.texture_def.extents
    }

    pub fn array_length(&self) -> u32 {
        self.texture_def.array_length
    }

    pub fn vk_aspect_mask(&self) -> vk::ImageAspectFlags {
        self.aspect_mask
    }

    pub fn vk_image(&self) -> vk::Image {
        self.image.image
    }

    pub fn vk_allocation(&self) -> Option<vk_mem::Allocation> {
        self.image.allocation
    }

    pub fn device_context(&self) -> &RafxDeviceContextVulkan {
        &self.device_context
    }

    // Color/Depth
    pub fn vk_srv_view(&self) -> Option<vk::ImageView> {
        self.srv_view
    }

    // Stencil-only
    pub fn vk_srv_view_stencil(&self) -> Option<vk::ImageView> {
        self.srv_view_stencil
    }

    // Mip chain
    pub fn vk_uav_views(&self) -> &[vk::ImageView] {
        &self.uav_views
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureVulkan> {
        Self::from_existing(device_context, None, texture_def)
    }

    // This path is mostly so we can wrap a provided swapchain image
    pub fn from_existing(
        device_context: &RafxDeviceContextVulkan,
        existing_image: Option<RafxRawImageVulkan>,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureVulkan> {
        texture_def.verify();

        // if RW texture, create image viewsper mip, otherwise none?

        //
        // Determine desired image type
        //
        let dimensions = texture_def
            .dimensions
            .determine_dimensions(texture_def.extents);
        let image_type = match dimensions {
            RafxTextureDimensions::Dim1D => vk::ImageType::TYPE_1D,
            RafxTextureDimensions::Dim2D => vk::ImageType::TYPE_2D,
            RafxTextureDimensions::Dim3D => vk::ImageType::TYPE_3D,
            RafxTextureDimensions::Auto => panic!("dimensions() should not return auto"),
        };

        let is_cubemap = texture_def
            .resource_type
            .contains(RafxResourceType::TEXTURE_CUBE);
        let format_vk = texture_def.format.into();

        // create the image
        let image = if let Some(existing_image) = existing_image {
            existing_image
        } else {
            //
            // Determine image usage flags
            //
            let mut usage_flags =
                super::util::resource_type_image_usage_flags(texture_def.resource_type);
            if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_COLOR)
            {
                usage_flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
            } else if texture_def
                .resource_type
                .intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL)
            {
                usage_flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
            }

            if usage_flags.intersects(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::STORAGE) {
                usage_flags |=
                    vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST;
            }

            //
            // Determine image create flags
            //
            let mut create_flags = vk::ImageCreateFlags::empty();
            if is_cubemap {
                create_flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE;
            }
            if image_type == vk::ImageType::TYPE_3D {
                create_flags |= vk::ImageCreateFlags::TYPE_2D_ARRAY_COMPATIBLE_KHR
            }

            //TODO: Could check vkGetPhysicalDeviceFormatProperties for if we support the format for
            // the various ways we might use it

            let allocation_create_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::GpuOnly,
                flags: vk_mem::AllocationCreateFlags::NONE,
                required_flags: vk::MemoryPropertyFlags::empty(),
                preferred_flags: vk::MemoryPropertyFlags::empty(),
                memory_type_bits: 0, // Do not exclude any memory types
                pool: None,
                user_data: None,
            };

            let extent = vk::Extent3D {
                width: texture_def.extents.width,
                height: texture_def.extents.height,
                depth: texture_def.extents.depth,
            };

            let image_create_info = vk::ImageCreateInfo::builder()
                .image_type(image_type)
                .extent(extent)
                .mip_levels(texture_def.mip_count)
                .array_layers(texture_def.array_length)
                .format(format_vk)
                .tiling(vk::ImageTiling::OPTIMAL)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .usage(usage_flags)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .samples(texture_def.sample_count.into())
                .flags(create_flags);

            //let allocator = device.allocator().clone();
            let (image, allocation, _allocation_info) = device_context
                .allocator()
                .create_image(&image_create_info, &allocation_create_info)
                .map_err(|_| {
                    log::error!("Error creating image");
                    vk::Result::ERROR_UNKNOWN
                })?;

            RafxRawImageVulkan {
                image,
                allocation: Some(allocation),
            }
        };

        let mut image_view_type = if image_type == vk::ImageType::TYPE_1D {
            if texture_def.array_length > 1 {
                vk::ImageViewType::TYPE_1D_ARRAY
            } else {
                vk::ImageViewType::TYPE_1D
            }
        } else if image_type == vk::ImageType::TYPE_2D {
            if is_cubemap {
                if texture_def.array_length > 6 {
                    vk::ImageViewType::CUBE_ARRAY
                } else {
                    vk::ImageViewType::CUBE
                }
            } else {
                if texture_def.array_length > 1 {
                    vk::ImageViewType::TYPE_2D_ARRAY
                } else {
                    vk::ImageViewType::TYPE_2D
                }
            }
        } else {
            assert_eq!(image_type, vk::ImageType::TYPE_3D);
            assert_eq!(texture_def.array_length, 1);
            vk::ImageViewType::TYPE_3D
        };

        //SRV
        let aspect_mask = super::util::image_format_to_aspect_mask(texture_def.format);
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(aspect_mask)
            .base_array_layer(0)
            .layer_count(texture_def.array_length)
            .base_mip_level(0)
            .level_count(texture_def.mip_count);

        let mut image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image.image)
            .view_type(image_view_type)
            .format(format_vk)
            .components(vk::ComponentMapping::default())
            .subresource_range(*subresource_range);

        // Create SRV without stencil
        let srv_view = if texture_def
            .resource_type
            .intersects(RafxResourceType::TEXTURE)
        {
            image_view_create_info.subresource_range.aspect_mask &= !vk::ImageAspectFlags::STENCIL;
            unsafe {
                Some(
                    device_context
                        .device()
                        .create_image_view(&*image_view_create_info, None)?,
                )
            }
        } else {
            None
        };

        // Create stencil-only SRV
        let srv_view_stencil = if texture_def
            .resource_type
            .intersects(RafxResourceType::TEXTURE_READ_WRITE)
            && aspect_mask.intersects(vk::ImageAspectFlags::STENCIL)
        {
            image_view_create_info.subresource_range.aspect_mask = vk::ImageAspectFlags::STENCIL;
            unsafe {
                Some(
                    device_context
                        .device()
                        .create_image_view(&*image_view_create_info, None)?,
                )
            }
        } else {
            None
        };

        // UAV
        let uav_views = if texture_def
            .resource_type
            .intersects(RafxResourceType::TEXTURE_READ_WRITE)
        {
            if image_view_type == vk::ImageViewType::CUBE_ARRAY
                || image_view_type == vk::ImageViewType::CUBE
            {
                image_view_type = vk::ImageViewType::TYPE_2D_ARRAY;
            }

            image_view_create_info.view_type = image_view_type;
            image_view_create_info.subresource_range.level_count = 1;

            let mut uav_views = Vec::with_capacity(texture_def.mip_count as usize);
            for i in 0..texture_def.mip_count {
                image_view_create_info.subresource_range.base_mip_level = i;
                unsafe {
                    uav_views.push(
                        device_context
                            .device()
                            .create_image_view(&*image_view_create_info, None)?,
                    );
                }
            }

            uav_views
        } else {
            vec![]
        };

        Ok(RafxTextureVulkan {
            texture_def: texture_def.clone(),
            device_context: device_context.clone(),
            image,
            aspect_mask,
            srv_view,
            srv_view_stencil,
            uav_views,
        })
    }
}
