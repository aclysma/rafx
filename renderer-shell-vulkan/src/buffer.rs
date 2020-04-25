use ash::vk;
use std::mem;
use super::Align;
use std::mem::ManuallyDrop;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;

/// Represents a vulkan buffer (vertex, index, image, etc.)
pub struct VkBuffer {
    pub device: ash::Device, // This struct is not responsible for releasing this
    pub buffer: vk::Buffer,
    pub buffer_memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

impl VkBuffer {
    pub fn new(
        logical_device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        usage: vk::BufferUsageFlags,
        required_property_flags: vk::MemoryPropertyFlags,
        size: vk::DeviceSize,
    ) -> VkResult<Self> {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { logical_device.create_buffer(&buffer_info, None)? };

        let buffer_memory_req = unsafe { logical_device.get_buffer_memory_requirements(buffer) };

        //TODO: Return error
        let buffer_memory_index = super::util::find_memorytype_index(
            &buffer_memory_req,
            device_memory_properties,
            required_property_flags,
        )
        .expect("Unable to find suitable memorytype for the buffer.");

        let buffer_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(buffer_memory_req.size)
            .memory_type_index(buffer_memory_index);

        let buffer_memory = unsafe { logical_device.allocate_memory(&buffer_allocate_info, None)? };

        unsafe { logical_device.bind_buffer_memory(buffer, buffer_memory, 0)? }

        Ok(VkBuffer {
            device: logical_device.clone(),
            buffer,
            buffer_memory,
            size,
        })
    }

    pub fn new_from_slice_device_local<T: Copy>(
        logical_device: &ash::Device,
        device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        usage: vk::BufferUsageFlags,
        data: &[T],
    ) -> VkResult<ManuallyDrop<VkBuffer>> {
        let vertex_buffer_size = data.len() as u64 * std::mem::size_of::<T>() as u64;

        let mut staging_buffer = super::VkBuffer::new(
            logical_device,
            &device_memory_properties,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            vertex_buffer_size,
        )?;

        staging_buffer.write_to_host_visible_buffer(data)?;

        let device_buffer = super::VkBuffer::new(
            &logical_device,
            &device_memory_properties,
            vk::BufferUsageFlags::TRANSFER_DST | usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            vertex_buffer_size,
        )?;

        VkBuffer::copy_buffer(
            logical_device,
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
        let ptr = unsafe {
            self.device.map_memory(
                self.buffer_memory,
                0, // offset
                self.size,
                vk::MemoryMapFlags::empty(),
            )?
        };

        let required_alignment = mem::align_of::<T>() as u64;
        let mut align = unsafe { Align::new(ptr, required_alignment, self.size) };

        align.copy_from_slice(data);

        unsafe {
            self.device.unmap_memory(self.buffer_memory);
        }

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
                let buffer_copy_info = [vk::BufferCopy::builder().size(src.size).build()];

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
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.buffer_memory, None);
        }

        trace!("destroyed VkBuffer");
    }
}
