use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use crate::{util, VkDevice};
use std::sync::Arc;
use std::mem::ManuallyDrop;
use crate::device::VkDeviceContext;
use core::fmt;

pub struct VkImage {
    pub device_context: VkDeviceContext,
    pub image: vk::Image,
    pub extent: vk::Extent3D,
    pub allocation: vk_mem::Allocation,
    pub allocation_info: vk_mem::AllocationInfo,
}

impl fmt::Debug for VkImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VkImage")
            .field("image", &self.image)
            .field("extent", &self.extent)
            .finish()
    }
}

impl VkImage {
    pub fn new(
        device_context: &VkDeviceContext,
        memory_usage: vk_mem::MemoryUsage,
        image_usage: vk::ImageUsageFlags,
        extent: vk::Extent3D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        required_property_flags: vk::MemoryPropertyFlags,
    ) -> VkResult<Self> {
        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            flags: vk_mem::AllocationCreateFlags::NONE,
            required_flags: required_property_flags,
            preferred_flags: vk::MemoryPropertyFlags::empty(),
            memory_type_bits: 0, // Do not exclude any memory types
            pool: None,
            user_data: None,
        };

        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(image_usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        //let allocator = device.allocator().clone();
        let (image, allocation, allocation_info) = device_context.allocator().create_image(&image_create_info, &allocation_create_info)
            .map_err(|_| vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)?;

        Ok(VkImage {
            device_context: device_context.clone(),
            image,
            extent,
            allocation,
            allocation_info,
        })
    }
}

impl Drop for VkImage {
    fn drop(&mut self) {
        log::debug!("destroying VkImage");

        unsafe {
            unsafe {
                self.device_context.allocator().destroy_image(self.image, &self.allocation);
            }
        }

        log::debug!("destroyed VkImage");
    }
}
