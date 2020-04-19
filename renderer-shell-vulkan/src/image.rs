use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

use crate::util;

pub struct VkImage {
    pub device: ash::Device, // This struct is not responsible for releasing this
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub extent: vk::Extent3D,
}

impl VkImage {
    pub fn new(
        logical_device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        extent: vk::Extent3D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        required_property_flags: vk::MemoryPropertyFlags,
    ) -> VkResult<Self> {
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let image = unsafe { logical_device.create_image(&image_create_info, None)? };

        let image_memory_req = unsafe { logical_device.get_image_memory_requirements(image) };

        //TODO: Better error handling here
        let image_memory_index = util::find_memorytype_index(
            &image_memory_req,
            device_memory_properties,
            required_property_flags,
        )
        .expect("Unable to find suitable memorytype for the vertex buffer.");

        let image_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(image_memory_req.size)
            .memory_type_index(image_memory_index);

        let image_memory = unsafe { logical_device.allocate_memory(&image_allocate_info, None)? };

        unsafe {
            logical_device.bind_image_memory(image, image_memory, 0)?;
        }

        Ok(VkImage {
            device: logical_device.clone(),
            image,
            image_memory,
            extent,
        })
    }
}

impl Drop for VkImage {
    fn drop(&mut self) {
        log::debug!("destroying VkImage");

        unsafe {
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.image_memory, None);
        }

        log::debug!("destroyed VkImage");
    }
}
