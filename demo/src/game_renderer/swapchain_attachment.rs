use ash::vk;
use ash::prelude::VkResult;
use ash::extensions::khr;

use ash::version::{DeviceV1_0, InstanceV1_0};

use renderer::vulkan::{PresentMode, VkDeviceContext, VkImage, MsaaLevel, SwapchainInfo};
use renderer::assets::resources::{ResourceArc, ImageViewResource, ResourceLookupSet};
use super::Window;
use std::mem::ManuallyDrop;
use renderer::assets::vk_description as dsc;

pub struct RenderpassAttachmentImage {
    pub msaa_image: Option<ResourceArc<ImageViewResource>>,
    pub resolved_image: ResourceArc<ImageViewResource>,
}

impl RenderpassAttachmentImage {
    pub fn new(
        resources: &mut ResourceLookupSet,
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        format: vk::Format,
        image_aspect_flags: vk::ImageAspectFlags,
        msaa_image_usage: vk::ImageUsageFlags,
        resolved_image_usage: vk::ImageUsageFlags,
        msaa_level: MsaaLevel,
    ) -> VkResult<Self> {
        let msaa_image = if msaa_level != MsaaLevel::Sample1 {
            Some(Self::create_resource(
                resources,
                device_context,
                swapchain_info,
                format,
                image_aspect_flags,
                msaa_image_usage,
                msaa_level,
            )?)
        } else {
            None
        };

        let resolved_image = Self::create_resource(
            resources,
            device_context,
            swapchain_info,
            format,
            image_aspect_flags,
            resolved_image_usage,
            MsaaLevel::Sample1,
        )?;

        Ok(RenderpassAttachmentImage {
            msaa_image,
            resolved_image,
        })
    }

    fn create_view_meta(
        format: vk::Format,
        image_aspect_flags: vk::ImageAspectFlags,
    ) -> dsc::ImageViewMeta {
        dsc::ImageViewMeta::default_2d_no_mips_or_layers(
            format.into(),
            dsc::ImageAspectFlags::from_bits(image_aspect_flags.as_raw()).unwrap(),
        )
    }

    fn create_image(
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        format: vk::Format,
        image_usage: vk::ImageUsageFlags,
        msaa_level: MsaaLevel,
    ) -> VkResult<ManuallyDrop<VkImage>> {
        let extents = vk::Extent3D {
            width: swapchain_info.extents.width,
            height: swapchain_info.extents.height,
            depth: 1,
        };

        let image = VkImage::new(
            device_context,
            vk_mem::MemoryUsage::GpuOnly,
            image_usage,
            extents,
            format,
            vk::ImageTiling::OPTIMAL,
            msaa_level.into(),
            1,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        Ok(ManuallyDrop::new(image))
    }

    pub fn create_resource(
        resources: &mut ResourceLookupSet,
        device_context: &VkDeviceContext,
        swapchain_info: &SwapchainInfo,
        format: vk::Format,
        image_aspect_flags: vk::ImageAspectFlags,
        image_usage: vk::ImageUsageFlags,
        msaa_level: MsaaLevel,
    ) -> VkResult<ResourceArc<ImageViewResource>> {
        let image_view_meta = Self::create_view_meta(format.into(), image_aspect_flags);

        let image = Self::create_image(
            device_context,
            swapchain_info,
            format,
            image_usage,
            msaa_level,
        )?;

        let image = resources.insert_image(image);
        resources.get_or_create_image_view(&image, &image_view_meta)
    }

    //
    // The "target" image/image view are the resources that should be written to and may or may not
    // be MSAA
    //
    pub fn target_image(&self) -> vk::Image {
        self.target_resource()
            .get_raw()
            .image
            .get_raw()
            .image
            .image
    }

    pub fn target_image_view(&self) -> vk::ImageView {
        self.target_resource().get_raw().image_view
    }

    pub fn target_resource(&self) -> &ResourceArc<ImageViewResource> {
        if let Some(msaa_image) = &self.msaa_image {
            msaa_image
        } else {
            &self.resolved_image
        }
    }

    //
    // The "resolved" image/image view are the resources that should be read from. Either it will be
    // resolved from the MSAA image, or the target image will have been the resolved image from the
    // start
    //
    pub fn resolved_image(&self) -> vk::Image {
        self.resolved_image.get_raw().image.get_raw().image.image
    }

    pub fn resolved_image_view(&self) -> vk::ImageView {
        self.resolved_image.get_raw().image_view
    }

    pub fn resolved_resource(&self) -> &ResourceArc<ImageViewResource> {
        &self.resolved_image
    }
}
