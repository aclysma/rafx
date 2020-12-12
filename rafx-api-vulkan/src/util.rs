use ash::vk;

/// Loads a shader into a buffer
pub use ash::util::read_spv;

pub fn round_size_up_to_alignment_u32(
    size: u32,
    required_alignment: u32,
) -> u32 {
    assert!(required_alignment > 0);
    ((size + required_alignment - 1) / required_alignment) * required_alignment
}

pub fn round_size_up_to_alignment(
    size: vk::DeviceSize,
    required_alignment: vk::DeviceSize,
) -> vk::DeviceSize {
    assert!(required_alignment > 0);
    ((size + required_alignment - 1) / required_alignment) * required_alignment
}

pub fn any_as_bytes<T: Copy>(data: &T) -> &[u8] {
    let ptr: *const T = data;
    let ptr = ptr as *const u8;
    let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of::<T>()) };

    slice
}

// Don't actually do this in shipping code
/*
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
*/
