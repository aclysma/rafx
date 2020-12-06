use ash::prelude::VkResult;
use ash::vk;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use rafx_shell_vulkan::VkBuffer;
use rafx_shell_vulkan::{VkDeviceContext, VkTransferUpload, VkTransferUploadState};

use image::{GenericImageView, ImageFormat};
use rafx_shell_vulkan::VkImage;
use std::sync::{Arc, Mutex};

#[derive(Copy, Clone, Debug)]
pub enum ColorSpace {
    Srgb,
    Linear,
}

#[derive(Copy, Clone, Debug)]
pub struct DecodedTextureMipInfo {
    pub mip_level_count: u32,
}

#[derive(Copy, Clone, Debug)]
pub enum DecodedTextureMips {
    // No mips - this should only be set if mip_level_count == 1
    None,

    // Mips should be generated from the loaded data at runtime
    Runtime(DecodedTextureMipInfo),

    // Mips, if any, are already computed and included in the loaded data
    Precomputed(DecodedTextureMipInfo),
}

impl DecodedTextureMips {
    pub fn mip_level_count(&self) -> u32 {
        match self {
            DecodedTextureMips::None => 1,
            DecodedTextureMips::Runtime(info) => info.mip_level_count,
            DecodedTextureMips::Precomputed(info) => info.mip_level_count,
        }
    }
}

#[derive(Clone)]
pub struct DecodedTexture {
    pub width: u32,
    pub height: u32,
    pub color_space: ColorSpace,
    pub mips: DecodedTextureMips,
    pub data: Vec<u8>,
}

// Provides default settings for an image that's loaded without metadata specifying mip settings
pub fn default_mip_settings_for_image(
    width: u32,
    height: u32,
) -> DecodedTextureMips {
    let max_dimension = std::cmp::max(width, height);
    let mip_level_count = (max_dimension as f32).log2().floor() as u32 + 1;
    let decoded_texture_mip_info = DecodedTextureMipInfo { mip_level_count };

    DecodedTextureMips::Runtime(decoded_texture_mip_info)

    //DecodedTextureMips::None
}

pub fn decode_texture(
    buf: &[u8],
    format: ImageFormat,
) -> DecodedTexture {
    let image_data = image::load_from_memory_with_format(buf, format).unwrap();
    let dimensions = image_data.dimensions();
    let image_data = image_data.to_rgba8().into_raw();
    let decoded_texture_mip_info = default_mip_settings_for_image(dimensions.0, dimensions.1);

    DecodedTexture {
        width: dimensions.0,
        height: dimensions.1,
        mips: decoded_texture_mip_info,
        data: image_data,
        color_space: ColorSpace::Srgb,
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
            dst_stage: vk::PipelineStageFlags::BOTTOM_OF_PIPE, // ignored, this is a release of resources to another queue
            src_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            dst_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        },
        ImageMemoryBarrierType::PostUploadDstQueue => SyncInfo {
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            src_stage: vk::PipelineStageFlags::TOP_OF_PIPE, // ignored, this is an acquire of resources from another queue
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

        let (mip_level_count, generate_mips) = match decoded_texture.mips {
            DecodedTextureMips::None => (1, false),
            DecodedTextureMips::Precomputed(_info) => unimplemented!(), //(info.mip_level_count, false),
            DecodedTextureMips::Runtime(info) => (info.mip_level_count, true),
        };

        // Arbitrary, not sure if there is any requirement
        const REQUIRED_ALIGNMENT: usize = 16;

        // Push data into the staging buffer
        let offset = upload.push(&decoded_texture.data, REQUIRED_ALIGNMENT)?;

        let mut image_usage = vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED;
        if generate_mips {
            image_usage |= vk::ImageUsageFlags::TRANSFER_SRC;
        };

        let format = match decoded_texture.color_space {
            ColorSpace::Linear => vk::Format::R8G8B8A8_UNORM,
            ColorSpace::Srgb => vk::Format::R8G8B8A8_SRGB,
        };

        // Allocate an image
        let image = ManuallyDrop::new(VkImage::new(
            device_context,
            vk_mem::MemoryUsage::GpuOnly,
            vk::ImageCreateFlags::empty(),
            image_usage,
            extent,
            format,
            vk::ImageTiling::OPTIMAL,
            vk::SampleCountFlags::TYPE_1,
            1,
            mip_level_count,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?);

        //
        // Write into the transfer command buffer
        // - transition destination memory to receive the data
        // - copy the data
        // - transition the destination to the graphics queue
        //

        // This just copies first mip of the chain
        cmd_image_memory_barrier(
            device_context.device(),
            upload.transfer_command_buffer(),
            &[image.image()],
            ImageMemoryBarrierType::PreUpload,
            transfer_queue_family_index,
            transfer_queue_family_index,
        );

        cmd_copy_buffer_to_image(
            device_context.device(),
            upload.transfer_command_buffer(),
            upload.staging_buffer().buffer(),
            offset,
            image.image(),
            &image.extent,
        );

        if generate_mips {
            // Generating mipmaps includes image barriers, so this function will handle writing the
            // image barriers required to pass from the transfer queue to the dst queue
            generate_mips_for_image(
                device_context,
                upload,
                transfer_queue_family_index,
                dst_queue_family_index,
                &image,
                mip_level_count,
            );
        } else {
            cmd_image_memory_barrier(
                device_context.device(),
                upload.transfer_command_buffer(),
                &[image.image()],
                ImageMemoryBarrierType::PostUploadTransferQueue,
                transfer_queue_family_index,
                dst_queue_family_index,
            );

            cmd_image_memory_barrier(
                device_context.device(),
                upload.dst_command_buffer(),
                &[image.image()],
                ImageMemoryBarrierType::PostUploadDstQueue,
                transfer_queue_family_index,
                dst_queue_family_index,
            );
        }

        images.push(image);
    }

    Ok(images)
}

fn generate_mips_for_image(
    device_context: &VkDeviceContext,
    upload: &mut VkTransferUpload,
    transfer_queue_family_index: u32,
    dst_queue_family_index: u32,
    image: &ManuallyDrop<VkImage>,
    mip_level_count: u32,
) {
    let first_mip_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .layer_count(1)
        .level_count(1)
        .build();

    transition_for_mipmap(
        device_context.device(),
        upload.transfer_command_buffer(),
        image.image(),
        vk::AccessFlags::TRANSFER_WRITE,
        vk::AccessFlags::TRANSFER_READ,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::TRANSFER,
        transfer_queue_family_index,
        dst_queue_family_index,
        &first_mip_range,
    );

    transition_for_mipmap(
        device_context.device(),
        upload.dst_command_buffer(),
        image.image(),
        vk::AccessFlags::TRANSFER_WRITE,
        vk::AccessFlags::TRANSFER_READ,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::TRANSFER,
        transfer_queue_family_index,
        dst_queue_family_index,
        &first_mip_range,
    );

    do_generate_mips_for_image(
        device_context,
        upload.dst_command_buffer(),
        dst_queue_family_index,
        &image,
        mip_level_count,
    );

    let all_mips_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .layer_count(1)
        .level_count(mip_level_count)
        .build();

    // Everything is in transfer read mode, transition it to our final layout
    transition_for_mipmap(
        device_context.device(),
        upload.dst_command_buffer(),
        image.image(),
        vk::AccessFlags::TRANSFER_READ,
        vk::AccessFlags::SHADER_READ,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        dst_queue_family_index,
        dst_queue_family_index,
        &all_mips_range,
    );
}

fn do_generate_mips_for_image(
    device_context: &VkDeviceContext,
    command_buffer: vk::CommandBuffer,
    queue_family_index: u32, // queue family that will do mip generation
    image: &ManuallyDrop<VkImage>,
    mip_level_count: u32,
) {
    log::debug!("Generating mipmaps");

    // Walk through each mip level n:
    // - put level n+1 into write mode
    // - blit from n to n+1
    // - put level n+1 into read mode
    for dst_level in 1..mip_level_count {
        log::trace!("Generating mipmap level {}", dst_level);
        let src_level = dst_level - 1;

        let src_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .layer_count(1)
            .mip_level(src_level);

        let src_offsets = [
            vk::Offset3D::default(),
            vk::Offset3D::builder()
                .x((image.extent.width as i32 >> src_level as i32).max(1))
                .y((image.extent.height as i32 >> src_level as i32).max(1))
                .z(1)
                .build(),
        ];

        let dst_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .layer_count(1)
            .mip_level(dst_level);

        let dst_offsets = [
            vk::Offset3D::default(),
            vk::Offset3D::builder()
                .x((image.extent.width as i32 >> dst_level as i32).max(1))
                .y((image.extent.height as i32 >> dst_level as i32).max(1))
                .z(1)
                .build(),
        ];

        log::trace!("src {:?}", src_offsets[1]);
        log::trace!("dst {:?}", dst_offsets[1]);

        let mip_subrange = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(dst_level)
            .level_count(1)
            .layer_count(1);

        log::trace!("  transition to write");
        transition_for_mipmap(
            device_context.device(),
            command_buffer,
            image.image(),
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            queue_family_index,
            queue_family_index,
            &mip_subrange,
        );

        let image_blit = vk::ImageBlit::builder()
            .src_offsets(src_offsets)
            .src_subresource(*src_subresource)
            .dst_offsets(dst_offsets)
            .dst_subresource(*dst_subresource);

        log::trace!("  blit");
        unsafe {
            device_context.device().cmd_blit_image(
                command_buffer,
                image.image(),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image.image(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[*image_blit],
                vk::Filter::LINEAR,
            );
        }

        log::trace!("  transition to read");
        transition_for_mipmap(
            device_context.device(),
            command_buffer,
            image.image(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::TRANSFER_READ,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::TRANSFER,
            queue_family_index,
            queue_family_index,
            &mip_subrange,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn transition_for_mipmap(
    logical_device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    src_access_mask: vk::AccessFlags,
    dst_access_mask: vk::AccessFlags,
    src_layout: vk::ImageLayout,
    dst_layout: vk::ImageLayout,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
    src_queue_family: u32,
    dst_queue_family: u32,
    subresource_range: &vk::ImageSubresourceRange,
) {
    let barrier = vk::ImageMemoryBarrier::builder()
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask)
        .old_layout(src_layout)
        .new_layout(dst_layout)
        .src_queue_family_index(src_queue_family)
        .dst_queue_family_index(dst_queue_family)
        .image(image)
        .subresource_range(*subresource_range)
        .build();

    unsafe {
        logical_device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            vk::DependencyFlags::BY_REGION,
            &[],
            &[],
            &[barrier],
        );
    }
}

pub fn load_images(
    device_context: &VkDeviceContext,
    transfer_queue_family_index: u32,
    transfer_queue: &Arc<Mutex<vk::Queue>>,
    dst_queue_family_index: u32,
    dst_queue: &Arc<Mutex<vk::Queue>>,
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
