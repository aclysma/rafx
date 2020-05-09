use std::mem;
use ash::vk;
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{
    VkDevice, VkUpload, VkUploadState, VkTransferUpload, VkTransferUploadState, VkDeviceContext,
};
use renderer_shell_vulkan::VkSwapchain;
use renderer_shell_vulkan::offset_of;
use renderer_shell_vulkan::SwapchainInfo;
use renderer_shell_vulkan::VkQueueFamilyIndices;
use renderer_shell_vulkan::VkBuffer;
use renderer_shell_vulkan::util;

use renderer_shell_vulkan::VkImage;
use image::error::ImageError::Decoding;
use std::process::exit;
use image::{GenericImageView, ImageFormat};
use ash::vk::ShaderStageFlags;

#[derive(Clone)]
pub struct DecodedTexture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub fn decode_texture(
    buf: &[u8],
    format: ImageFormat,
) -> DecodedTexture {
    let example_image = image::load_from_memory_with_format(buf, format).unwrap();
    let dimensions = example_image.dimensions();
    let example_image = example_image.to_rgba().into_raw();
    DecodedTexture {
        width: dimensions.0,
        height: dimensions.1,
        data: example_image,
    }
}

#[derive(PartialEq)]
pub enum ImageMemoryBarrierType {
    PreUpload,
    PostUploadUnifiedQueues,
    PostUploadTransferQueue,
    PostUploadDstQueue,
}

pub fn cmd_image_memory_barrier(
    logical_device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    images: &[vk::Image],
    barrier_type: ImageMemoryBarrierType,
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
        src_layout: vk::ImageLayout,
        dst_layout: vk::ImageLayout,
    }

    let sync_info = match barrier_type {
        ImageMemoryBarrierType::PreUpload => SyncInfo {
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            src_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
            dst_stage: vk::PipelineStageFlags::TRANSFER,
            src_layout: vk::ImageLayout::UNDEFINED,
            dst_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        },
        ImageMemoryBarrierType::PostUploadUnifiedQueues => SyncInfo {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            src_stage: vk::PipelineStageFlags::TRANSFER,
            dst_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
            src_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            dst_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },
        ImageMemoryBarrierType::PostUploadTransferQueue => SyncInfo {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::empty(),
            src_stage: vk::PipelineStageFlags::TRANSFER,
            dst_stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            src_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            dst_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },
        ImageMemoryBarrierType::PostUploadDstQueue => SyncInfo {
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            src_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
            dst_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
            src_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            dst_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },
    };

    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);

    let barrier_infos: Vec<_> = images
        .iter()
        .map(|image| {
            vk::ImageMemoryBarrier::builder()
                .src_access_mask(sync_info.src_access_mask)
                .dst_access_mask(sync_info.dst_access_mask)
                .old_layout(sync_info.src_layout)
                .new_layout(sync_info.dst_layout)
                .src_queue_family_index(src_queue_family)
                .dst_queue_family_index(dst_queue_family)
                .image(*image)
                .subresource_range(*subresource_range)
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
            &[],
            &barrier_infos,
        );
    }
}

pub fn cmd_copy_buffer_to_image(
    logical_device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    buffer: vk::Buffer,
    offset: vk::DeviceSize,
    image: vk::Image,
    extent: &vk::Extent3D,
) {
    let image_subresource = vk::ImageSubresourceLayers::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1);

    let image_copy = vk::BufferImageCopy::builder()
        .buffer_offset(offset)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(*image_subresource)
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(*extent);

    unsafe {
        logical_device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[*image_copy],
        );
    }
}

pub fn enqueue_load_images(
    device_context: &VkDeviceContext,
    upload: &mut VkTransferUpload,
    transfer_queue_family_index: u32,
    dst_queue_family_index: u32,
    decoded_textures: &[DecodedTexture],
) -> VkResult<Vec<ManuallyDrop<VkImage>>> {
    let mut images = Vec::with_capacity(decoded_textures.len());

    for decoded_texture in decoded_textures {
        let extent = vk::Extent3D {
            width: decoded_texture.width,
            height: decoded_texture.height,
            depth: 1,
        };

        // Arbitrary, not sure if there is any requirement
        const REQUIRED_ALIGNMENT : usize = 16;

        // Push data into the staging buffer
        let offset = upload.push(&decoded_texture.data, REQUIRED_ALIGNMENT)?;

        // Allocate an image
        let image = ManuallyDrop::new(VkImage::new(
            device_context,
            vk_mem::MemoryUsage::GpuOnly,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            extent,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageTiling::OPTIMAL,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?);

        cmd_image_memory_barrier(
            device_context.device(),
            upload.transfer_command_buffer(),
            &[image.image],
            ImageMemoryBarrierType::PreUpload,
            transfer_queue_family_index,
            transfer_queue_family_index,
        );

        cmd_copy_buffer_to_image(
            device_context.device(),
            upload.transfer_command_buffer(),
            upload.staging_buffer().buffer,
            offset,
            image.image,
            &image.extent,
        );

        cmd_image_memory_barrier(
            device_context.device(),
            upload.transfer_command_buffer(),
            &[image.image],
            ImageMemoryBarrierType::PostUploadTransferQueue,
            transfer_queue_family_index,
            dst_queue_family_index,
        );

        images.push(image);
    }

    for image in &images {
        cmd_image_memory_barrier(
            device_context.device(),
            upload.dst_command_buffer(),
            &[image.image],
            ImageMemoryBarrierType::PostUploadDstQueue,
            transfer_queue_family_index,
            dst_queue_family_index,
        );
    }

    Ok(images)
}

pub fn load_images(
    device_context: &VkDeviceContext,
    transfer_queue_family_index: u32,
    transfer_queue: vk::Queue,
    dst_queue_family_index: u32,
    dst_queue: vk::Queue,
    decoded_textures: &[DecodedTexture],
) -> VkResult<Vec<ManuallyDrop<VkImage>>> {
    let mut upload = VkTransferUpload::new(
        device_context,
        transfer_queue_family_index,
        dst_queue_family_index,
        1024 * 1024 * 16,
    )?;

    let images = enqueue_load_images(
        device_context,
        &mut upload,
        transfer_queue_family_index,
        dst_queue_family_index,
        decoded_textures,
    )?;

    upload.submit_transfer(transfer_queue)?;
    loop {
        if upload.state()? == VkTransferUploadState::PendingSubmitDstQueue {
            break;
        }
    }

    upload.submit_dst(dst_queue)?;
    loop {
        if upload.state()? == VkTransferUploadState::Complete {
            break;
        }
    }

    Ok(images)
}







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
    size: u64
) {
    let buffer_copy = vk::BufferCopy::builder()
        .src_offset(src_buffer_offset)
        .dst_offset(0)
        .size(size);

    unsafe {
        logical_device.cmd_copy_buffer(
            command_buffer,
            src_buffer,
            dst_buffer,
            &[*buffer_copy]
        );
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
        const REQUIRED_ALIGNMENT : usize = 16;

        // Push data into the staging buffer
        let offset = upload.push(&data_array, REQUIRED_ALIGNMENT)?;
        let size = data_array.len() as u64;

        // Allocate an image
        let dst_buffer = ManuallyDrop::new(VkBuffer::new(
            device_context,
            vk_mem::MemoryUsage::GpuOnly,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            size,
        )?);

        cmd_copy_buffer_to_buffer(
            device_context.device(),
            upload.transfer_command_buffer(),
            upload.staging_buffer().buffer,
            dst_buffer.buffer,
            offset,
            size,
        );

        cmd_buffer_memory_barrier(
            device_context.device(),
            upload.transfer_command_buffer(),
            &[dst_buffer.buffer],
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
            &[dst_buffer.buffer],
            BufferMemoryBarrierType::PostUploadDstQueue,
            transfer_queue_family_index,
            dst_queue_family_index,
        );
    }

    Ok(dst_buffers)
}