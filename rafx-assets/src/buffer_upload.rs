use ash::prelude::VkResult;
use ash::vk;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use rafx_shell_vulkan::VkBuffer;
use rafx_shell_vulkan::{VkDeviceContext, VkTransferUpload};

#[derive(PartialEq)]
pub enum BufferMemoryBarrierType {
    PostUploadUnifiedQueues,
    PostUploadTransferQueue,
    PostUploadDstQueue,
}

pub fn cmd_buffer_memory_barrier(
    logical_device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    buffers: &[vk::Buffer],
    barrier_type: BufferMemoryBarrierType,
    mut src_queue_family: u32,
    mut dst_queue_family: u32,
) {
    if src_queue_family == dst_queue_family {
        src_queue_family = vk::QUEUE_FAMILY_IGNORED;
        dst_queue_family = vk::QUEUE_FAMILY_IGNORED;
    }

    struct SyncInfo {
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
    }

    let sync_info = match barrier_type {
        BufferMemoryBarrierType::PostUploadUnifiedQueues => SyncInfo {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            src_stage: vk::PipelineStageFlags::TRANSFER,
            dst_stage: vk::PipelineStageFlags::VERTEX_INPUT,
        },
        BufferMemoryBarrierType::PostUploadTransferQueue => SyncInfo {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::empty(),
            src_stage: vk::PipelineStageFlags::TRANSFER,
            dst_stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        },
        BufferMemoryBarrierType::PostUploadDstQueue => SyncInfo {
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
            src_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
            dst_stage: vk::PipelineStageFlags::VERTEX_INPUT,
        },
    };

    let barrier_infos: Vec<_> = buffers
        .iter()
        .map(|buffer| {
            vk::BufferMemoryBarrier::builder()
                .src_access_mask(sync_info.src_access_mask)
                .dst_access_mask(sync_info.dst_access_mask)
                .src_queue_family_index(src_queue_family)
                .dst_queue_family_index(dst_queue_family)
                .buffer(*buffer)
                .size(vk::WHOLE_SIZE)
                .offset(0)
                .build()
        })
        .collect();

    unsafe {
        logical_device.cmd_pipeline_barrier(
            command_buffer,
            sync_info.src_stage,
            sync_info.dst_stage,
            vk::DependencyFlags::BY_REGION,
            &[],
            &barrier_infos,
            &[],
        );
    }
}

pub fn cmd_copy_buffer_to_buffer(
    logical_device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    src_buffer: vk::Buffer,
    dst_buffer: vk::Buffer,
    src_buffer_offset: u64,
    size: u64,
) {
    let buffer_copy = vk::BufferCopy::builder()
        .src_offset(src_buffer_offset)
        .dst_offset(0)
        .size(size);

    unsafe {
        logical_device.cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &[*buffer_copy]);
    }
}

pub fn enqueue_load_buffers(
    device_context: &VkDeviceContext,
    upload: &mut VkTransferUpload,
    transfer_queue_family_index: u32,
    dst_queue_family_index: u32,
    data_arrays: &[Vec<u8>],
) -> VkResult<Vec<ManuallyDrop<VkBuffer>>> {
    let mut dst_buffers = Vec::with_capacity(data_arrays.len());

    for data_array in data_arrays {
        // Arbitrary, not sure if there is any requirement
        const REQUIRED_ALIGNMENT: usize = 16;

        // Push data into the staging buffer
        let offset = upload.push(&data_array, REQUIRED_ALIGNMENT)?;
        let size = data_array.len() as u64;

        // Allocate an image
        let dst_buffer = ManuallyDrop::new(VkBuffer::new(
            device_context,
            vk_mem::MemoryUsage::GpuOnly,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            size,
        )?);

        cmd_copy_buffer_to_buffer(
            device_context.device(),
            upload.transfer_command_buffer(),
            upload.staging_buffer().buffer(),
            dst_buffer.buffer(),
            offset,
            size,
        );

        cmd_buffer_memory_barrier(
            device_context.device(),
            upload.transfer_command_buffer(),
            &[dst_buffer.buffer()],
            BufferMemoryBarrierType::PostUploadTransferQueue,
            transfer_queue_family_index,
            dst_queue_family_index,
        );

        dst_buffers.push(dst_buffer);
    }

    for dst_buffer in &dst_buffers {
        cmd_buffer_memory_barrier(
            device_context.device(),
            upload.dst_command_buffer(),
            &[dst_buffer.buffer()],
            BufferMemoryBarrierType::PostUploadDstQueue,
            transfer_queue_family_index,
            dst_queue_family_index,
        );
    }

    Ok(dst_buffers)
}
