use ash::vk;
use std::mem;
use super::Align;
use std::mem::ManuallyDrop;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;
use crate::{VkDevice, VkUpload};
use std::sync::Arc;
use crate::device::VkDeviceContext;

#[derive(Copy, Clone, Debug)]
pub struct VkBufferRaw {
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
}

/// Represents a vulkan buffer (vertex, index, image, etc.)
pub struct VkBuffer {
    pub device_context: VkDeviceContext,
    pub allocation_info: vk_mem::AllocationInfo,
    pub raw: Option<VkBufferRaw>,
}

impl VkBuffer {
    pub fn size(&self) -> vk::DeviceSize {
        self.allocation_info.get_size() as vk::DeviceSize
    }

    pub fn new(
        device_context: &VkDeviceContext,
        memory_usage: vk_mem::MemoryUsage,
        buffer_usage: vk::BufferUsageFlags,
        required_property_flags: vk::MemoryPropertyFlags,
        size: vk::DeviceSize,
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

        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(buffer_usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        //TODO: Better way of handling allocator errors
        let (buffer, allocation, allocation_info) = device_context
            .allocator()
            .create_buffer(&buffer_info, &allocation_create_info)
            .map_err(|_| vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)?;

        let raw = VkBufferRaw { buffer, allocation };

        Ok(VkBuffer {
            device_context: device_context.clone(),
            allocation_info,
            raw: Some(raw)
        })
    }

    pub fn write_to_host_visible_buffer<T: Copy>(
        &mut self,
        data: &[T],
    ) -> VkResult<()> {
        let allocation = self.allocation();

        //TODO: Better way of handling allocator errors
        let ptr = self
            .device_context
            .allocator()
            .map_memory(&allocation)
            .map_err(|_| vk::Result::ERROR_MEMORY_MAP_FAILED)?
            as *mut std::ffi::c_void;

        let required_alignment = mem::align_of::<T>() as u64;
        let mut align = unsafe { Align::new(ptr, required_alignment, self.size()) };

        align.copy_from_slice(data);

        //TODO: Better way of handling allocator errors
        self.device_context
            .allocator()
            .unmap_memory(&allocation)
            .map_err(|_| vk::Result::ERROR_MEMORY_MAP_FAILED)?;

        // The staging buffer is coherent so flushing is not necessary

        Ok(())
    }

    pub fn buffer(&self) -> vk::Buffer {
        // Raw is only none if take_raw has not been called, and take_raw consumes the VkBuffer
        self.raw.unwrap().buffer
    }

    pub fn allocation(&self) -> vk_mem::Allocation {
        // Raw is only none if take_raw has not been called, and take_raw consumes the VkBuffer
        self.raw.unwrap().allocation
    }

    pub fn take_raw(mut self) -> Option<VkBufferRaw> {
        let mut raw = None;
        std::mem::swap(&mut raw, &mut self.raw);
        raw
    }
}

impl Drop for VkBuffer {
    fn drop(&mut self) {
        trace!("destroying VkBuffer");

        unsafe {
            if let Some(raw) = &self.raw {
                self.device_context
                    .allocator()
                    .destroy_buffer(raw.buffer, &raw.allocation);
            }
        }

        trace!("destroyed VkBuffer");
    }
}
