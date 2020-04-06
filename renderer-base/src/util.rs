use std::io;
use ash::vk;
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;

/// Find a memory type index that meets the requirements
pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    required_property_flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    for (index, ref memory_type) in memory_prop.memory_types.iter().enumerate() {
        let type_supported = (memory_req.memory_type_bits & (1 << index)) != 0;
        let flags_supported =
            (memory_type.property_flags & required_property_flags) == required_property_flags;

        if type_supported && flags_supported {
            return Some(index as u32);
        }
    }

    None
}

/// Loads a shader into a buffer
pub fn read_spv<R: io::Read + io::Seek>(x: &mut R) -> io::Result<Vec<u32>> {
    let size = x.seek(io::SeekFrom::End(0))?;
    if size % 4 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input length not divisible by 4",
        ));
    }
    if size > usize::max_value() as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "input too long"));
    }
    let words = (size / 4) as usize;
    let mut result = Vec::<u32>::with_capacity(words);
    x.seek(io::SeekFrom::Start(0))?;
    unsafe {
        x.read_exact(std::slice::from_raw_parts_mut(
            result.as_mut_ptr() as *mut u8,
            words * 4,
        ))?;
        result.set_len(words);
    }
    const MAGIC_NUMBER: u32 = 0x0723_0203;
    if !result.is_empty() && result[0] == MAGIC_NUMBER.swap_bytes() {
        for word in &mut result {
            *word = word.swap_bytes();
        }
    }
    if result.is_empty() || result[0] != MAGIC_NUMBER {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input missing SPIR-V magic number",
        ));
    }
    Ok(result)
}

/// Fires off a command buffer and then waits for the device to be idle
pub fn submit_single_use_command_buffer<F: Fn(vk::CommandBuffer)>(
    logical_device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    f: F,
) -> VkResult<()> {
    let alloc_info = vk::CommandBufferAllocateInfo::builder()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(command_pool)
        .command_buffer_count(1);

    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    let command_buffer = unsafe {
        let command_buffer = logical_device.allocate_command_buffers(&alloc_info)?[0];

        logical_device.begin_command_buffer(command_buffer, &begin_info)?;

        f(command_buffer);

        logical_device.end_command_buffer(command_buffer)?;

        command_buffer
    };

    let command_buffers = [command_buffer];
    let submit_info = vk::SubmitInfo::builder().command_buffers(&command_buffers);

    unsafe {
        logical_device.queue_submit(queue, &[submit_info.build()], vk::Fence::null())?;
        logical_device.device_wait_idle()?;

        logical_device.free_command_buffers(command_pool, &command_buffers);
    }

    Ok(())
}
