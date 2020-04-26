use ash::vk;
use std::mem;
use super::Align;
use std::mem::ManuallyDrop;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;
use crate::VkDevice;
use std::sync::Arc;
use crate::device::VkDeviceContext;

/// Represents a vulkan buffer (vertex, index, image, etc.)
pub struct VkBuffer {
    pub device_context: VkDeviceContext,
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
    pub allocation_info: vk_mem::AllocationInfo,
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
        let (buffer, allocation, allocation_info) =
            device_context.allocator().create_buffer(&buffer_info, &allocation_create_info)
                .map_err(|_| vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)?;

        Ok(VkBuffer {
            device_context: device_context.clone(),
            buffer,
            allocation,
            allocation_info
        })
    }

    pub fn new_from_slice_device_local<T: Copy>(
        device_context: &VkDeviceContext,
        buffer_usage: vk::BufferUsageFlags,
        required_property_flags: vk::MemoryPropertyFlags,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        data: &[T],
    ) -> VkResult<ManuallyDrop<VkBuffer>> {
        let vertex_buffer_size = data.len() as u64 * std::mem::size_of::<T>() as u64;

        let mut staging_buffer = super::VkBuffer::new(
            device_context,
            vk_mem::MemoryUsage::CpuOnly,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            vertex_buffer_size,
        )?;

        staging_buffer.write_to_host_visible_buffer(data)?;

        let device_buffer = super::VkBuffer::new(
            device_context,
            vk_mem::MemoryUsage::GpuOnly,
            vk::BufferUsageFlags::TRANSFER_DST | buffer_usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            vertex_buffer_size,
        )?;

        VkBuffer::copy_buffer(
            device_context.device(),
            queue,
            command_pool,
            &staging_buffer,
            &device_buffer,
        )?;

        Ok(ManuallyDrop::new(device_buffer))
    }

    pub fn write_to_host_visible_buffer<T: Copy>(
        &mut self,
        data: &[T],
    ) -> VkResult<()> {
        //TODO: Better way of handling allocator errors
        let ptr = self.device_context.allocator().map_memory(&self.allocation)
            .map_err(|_| vk::Result::ERROR_MEMORY_MAP_FAILED)?;

        let required_alignment = mem::align_of::<T>() as u64;
        let mut align = unsafe { Align::new(ptr, required_alignment, self.size()) };

        align.copy_from_slice(data);

        //TODO: Better way of handling allocator errors
        self.device_context.allocator().unmap_memory(&self.allocation)
            .map_err(|_| vk::Result::ERROR_MEMORY_MAP_FAILED)?;

        // The staging buffer is coherent so flushing is not necessary

        Ok(())
    }

    pub fn copy_buffer(
        logical_device: &ash::Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        src: &VkBuffer,
        dst: &VkBuffer,
    ) -> VkResult<()> {
        super::util::submit_single_use_command_buffer(
            logical_device,
            queue,
            command_pool,
            |command_buffer| {
                let buffer_copy_info = [vk::BufferCopy::builder().size(src.size()).build()];

                unsafe {
                    logical_device.cmd_copy_buffer(
                        command_buffer,
                        src.buffer,
                        dst.buffer,
                        &buffer_copy_info,
                    );
                }
            },
        )
    }
}

impl Drop for VkBuffer {
    fn drop(&mut self) {
        trace!("destroying VkBuffer");

        unsafe {
            self.device_context.allocator().destroy_buffer(self.buffer, &self.allocation);
        }

        trace!("destroyed VkBuffer");
    }
}
