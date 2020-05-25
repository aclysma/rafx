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
    always_mapped: bool
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
        Self::new_internal(
            device_context,
            memory_usage,
            buffer_usage,
            required_property_flags,
            size,
            false
        )
    }

    pub fn new_always_mapped(
        device_context: &VkDeviceContext,
        memory_usage: vk_mem::MemoryUsage,
        buffer_usage: vk::BufferUsageFlags,
        required_property_flags: vk::MemoryPropertyFlags,
        size: vk::DeviceSize,
    ) -> VkResult<Self> {
        Self::new_internal(
            device_context,
            memory_usage,
            buffer_usage,
            required_property_flags,
            size,
            true
        )
    }

    pub fn new_internal(
        device_context: &VkDeviceContext,
        memory_usage: vk_mem::MemoryUsage,
        buffer_usage: vk::BufferUsageFlags,
        required_property_flags: vk::MemoryPropertyFlags,
        size: vk::DeviceSize,
        always_mapped: bool
    ) -> VkResult<Self> {
        let mut flags = vk_mem::AllocationCreateFlags::NONE;
        if always_mapped {
            flags |= vk_mem::AllocationCreateFlags::MAPPED;
        }

        let allocation_create_info = vk_mem::AllocationCreateInfo {
            usage: memory_usage,
            flags,
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
            always_mapped,
            raw: Some(raw)
        })
    }

    pub fn write_to_host_visible_buffer<T: Copy>(
        &mut self,
        data: &[T],
    ) -> VkResult<()> {
        self.write_to_host_visible_buffer_with_offset(data, 0)
    }

    pub fn write_to_host_visible_buffer_with_offset<T: Copy>(
        &mut self,
        data: &[T],
        offset: u64,
    ) -> VkResult<()> {
        let allocation = self.allocation();

        let dst_bytes_available = self.size() - offset;

        let dst = if self.always_mapped {
            self.allocation_info.get_mapped_data()
        } else {
            self.device_context
                .allocator()
                .map_memory(&allocation)
                .map_err(|_| vk::Result::ERROR_MEMORY_MAP_FAILED)?
                as *mut u8
        };

        // let dst = unsafe { dst.add(offset as usize) };
        // let src = crate::util::any_as_bytes(&data);
        //
        // assert!(dst_bytes_available >= src.len() as u64);
        //
        // unsafe {
        //     dst.copy_from_nonoverlapping(src.as_ptr(), src.len());
        // }

        let required_alignment = mem::align_of::<T>() as u64;
        let mut align = unsafe { Align::new(dst as *mut std::ffi::c_void, required_alignment, self.size()) };
        align.copy_from_slice(data);

        if !self.always_mapped {
            //TODO: Better way of handling allocator errors
            self.device_context
                .allocator()
                .unmap_memory(&allocation)
                .map_err(|_| vk::Result::ERROR_MEMORY_MAP_FAILED)?;
        }


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
