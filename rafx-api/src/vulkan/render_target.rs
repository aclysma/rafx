use crate::vulkan::{RafxDeviceContextVulkan, RafxRawImageVulkan, RafxTextureVulkan};
use crate::{
    RafxRenderTarget, RafxRenderTargetDef, RafxResourceType, RafxResult, RafxTexture,
    RafxTextureDimensions,
};
use ash::version::DeviceV1_0;
use ash::vk;
use std::hash::{Hash, Hasher};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static RENDER_TARGET_NEXT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug)]
pub struct RafxRenderTargetVulkanInner {
    // It's a RafxTextureVulkan, but stored as RafxTexture so that we can return refs to it
    texture: RafxTexture,
    is_undefined_layout: AtomicBool,

    render_target_def: RafxRenderTargetDef,
    render_target_id: u32,
    view: vk::ImageView,
    view_slices: Vec<vk::ImageView>,
}

impl Drop for RafxRenderTargetVulkanInner {
    fn drop(&mut self) {
        let device = self.texture.vk_texture().unwrap().device_context().device();

        unsafe {
            device.destroy_image_view(self.view, None);

            for view_slice in &self.view_slices {
                device.destroy_image_view(*view_slice, None);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct RafxRenderTargetVulkan {
    inner: Arc<RafxRenderTargetVulkanInner>,
}

impl PartialEq for RafxRenderTargetVulkan {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.inner.render_target_id == other.inner.render_target_id
    }
}

impl Eq for RafxRenderTargetVulkan {}

impl Hash for RafxRenderTargetVulkan {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.inner.render_target_id.hash(state);
    }
}

impl RafxRenderTargetVulkan {
    pub fn render_target_def(&self) -> &RafxRenderTargetDef {
        &self.inner.render_target_def
    }

    pub fn texture(&self) -> &RafxTexture {
        &self.inner.texture
    }

    pub fn vk_texture(&self) -> &RafxTextureVulkan {
        self.inner.texture.vk_texture().unwrap()
    }

    pub fn render_target_vk_view(&self) -> vk::ImageView {
        self.inner.view
    }

    pub fn vk_aspect_mask(&self) -> vk::ImageAspectFlags {
        self.inner.texture.vk_texture().unwrap().vk_aspect_mask()
    }

    pub fn vk_image(&self) -> vk::Image {
        self.inner.texture.vk_texture().unwrap().vk_image()
    }

    pub fn render_target_slice_vk_view(
        &self,
        depth: u32,
        array_index: u16,
        mip_level: u8,
    ) -> vk::ImageView {
        assert!(
            depth == 0
                || self
                    .inner
                    .render_target_def
                    .resource_type
                    .intersects(RafxResourceType::RENDER_TARGET_DEPTH_SLICES)
        );
        assert!(
            array_index == 0
                || self
                    .inner
                    .render_target_def
                    .resource_type
                    .intersects(RafxResourceType::RENDER_TARGET_ARRAY_SLICES)
        );

        let def = &self.inner.render_target_def;
        let index = (mip_level as usize * def.array_length as usize * def.extents.depth as usize)
            + (array_index as usize * def.extents.depth as usize)
            + depth as usize;
        self.inner.view_slices[index]
    }

    // Used internally as part of the hash for creating/reusing framebuffers
    pub(crate) fn render_target_id(&self) -> u32 {
        self.inner.render_target_id
    }

    // Command buffers check this to see if an image needs to be transitioned from UNDEFINED
    pub(crate) fn take_is_undefined_layout(&self) -> bool {
        self.inner
            .is_undefined_layout
            .swap(false, Ordering::Relaxed)
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        render_target_def: &RafxRenderTargetDef,
    ) -> RafxResult<Self> {
        Self::from_existing(device_context, None, render_target_def)
    }

    pub fn from_existing(
        device_context: &RafxDeviceContextVulkan,
        existing_image: Option<RafxRawImageVulkan>,
        render_target_def: &RafxRenderTargetDef,
    ) -> RafxResult<Self> {
        render_target_def.verify();

        let texture_def = render_target_def.to_texture_def();

        //if has_depth {
        //TODO: Check the format is supported with vkGetPhysicalDeviceImageFormatProperties or VkSwapchain::choose_supported_format()
        // Either fail or default to something safe
        //}

        let texture =
            RafxTextureVulkan::from_existing(device_context, existing_image, &texture_def)?;

        let depth_array_size_multiple =
            render_target_def.extents.depth * render_target_def.array_length;

        let image_view_type = if render_target_def.dimensions != RafxTextureDimensions::Dim1D {
            if depth_array_size_multiple > 1 {
                vk::ImageViewType::TYPE_2D_ARRAY
            } else {
                vk::ImageViewType::TYPE_2D
            }
        } else {
            if depth_array_size_multiple > 1 {
                vk::ImageViewType::TYPE_1D_ARRAY
            } else {
                vk::ImageViewType::TYPE_1D
            }
        };

        //SRV
        let aspect_mask = super::util::image_format_to_aspect_mask(texture_def.format);
        let format_vk = render_target_def.format.into();
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(aspect_mask)
            .base_array_layer(0)
            .layer_count(depth_array_size_multiple)
            .base_mip_level(0)
            .level_count(1);

        let mut image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(texture.vk_image())
            .view_type(image_view_type)
            .format(format_vk)
            .components(vk::ComponentMapping::default())
            .subresource_range(*subresource_range);

        let view = unsafe {
            device_context
                .device()
                .create_image_view(&*image_view_create_info, None)?
        };

        let array_or_depth_slices = render_target_def.resource_type.intersects(
            RafxResourceType::RENDER_TARGET_ARRAY_SLICES
                | RafxResourceType::RENDER_TARGET_DEPTH_SLICES,
        );

        let mut view_slices = vec![];
        for i in 0..render_target_def.mip_count {
            image_view_create_info.subresource_range.base_mip_level = i;

            if array_or_depth_slices {
                for j in 0..depth_array_size_multiple {
                    image_view_create_info.subresource_range.layer_count = 1;
                    image_view_create_info.subresource_range.base_array_layer = j;
                    let view = unsafe {
                        device_context
                            .device()
                            .create_image_view(&*image_view_create_info, None)?
                    };
                    view_slices.push(view);
                }
            } else {
                let view = unsafe {
                    device_context
                        .device()
                        .create_image_view(&*image_view_create_info, None)?
                };
                view_slices.push(view);
            }
        }

        // Used for hashing framebuffers
        let render_target_id = RENDER_TARGET_NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let inner = RafxRenderTargetVulkanInner {
            texture: RafxTexture::Vk(texture),
            is_undefined_layout: AtomicBool::new(true),
            render_target_id,
            view,
            view_slices,
            render_target_def: render_target_def.clone(),
        };

        Ok(RafxRenderTargetVulkan {
            inner: Arc::new(inner),
        })
    }
}

impl Into<RafxRenderTarget> for RafxRenderTargetVulkan {
    fn into(self) -> RafxRenderTarget {
        RafxRenderTarget::Vk(self)
    }
}
