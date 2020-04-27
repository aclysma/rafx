use std::mem;
use ash::vk;
use ash::prelude::VkResult;
use std::ffi::CString;
use std::mem::ManuallyDrop;

use ash::version::DeviceV1_0;

use renderer_shell_vulkan::{VkDevice, VkUploader};
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

pub fn cmd_transition_image_layout(
    logical_device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    images: &[vk::Image],
    _format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) {
    struct SyncInfo {
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
    }

    let sync_info = match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => SyncInfo {
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            src_stage: vk::PipelineStageFlags::TOP_OF_PIPE,
            dst_stage: vk::PipelineStageFlags::TRANSFER,
        },
        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
            SyncInfo {
                src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                src_stage: vk::PipelineStageFlags::TRANSFER,
                dst_stage: vk::PipelineStageFlags::FRAGMENT_SHADER,
            }
        }
        _ => {
            // Layout transition not yet supported
            unimplemented!();
        }
    };

    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);

    let barrier_infos : Vec<_> = images.iter().map(|image| {
        vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(*image)
            .subresource_range(*subresource_range)
            .src_access_mask(sync_info.src_access_mask)
            .dst_access_mask(sync_info.dst_access_mask)
            .build()
    }).collect();

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

pub fn load_images(
    device: &VkDevice,
    queue: vk::Queue,
    decoded_textures: &[DecodedTexture],
) -> VkResult<Vec<ManuallyDrop<VkImage>>> {
    let mut uploader = VkUploader::new(device, device.queue_family_indices.graphics_queue_family_index, 1024*1024*16)?;

    let mut images = Vec::with_capacity(decoded_textures.len());

    for decoded_texture in decoded_textures {
        let extent = vk::Extent3D {
            width: decoded_texture.width,
            height: decoded_texture.height,
            depth: 1,
        };

        // Push data into the staging buffer
        let offset = uploader.push(&decoded_texture.data, std::mem::size_of::<usize>())?;

        // Allocate an image
        let image = ManuallyDrop::new(VkImage::new(
            &device.context,
            vk_mem::MemoryUsage::GpuOnly,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            extent,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageTiling::OPTIMAL,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?);

        cmd_transition_image_layout(
            device.device(),
            uploader.command_buffer(),
            &[image.image],
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        cmd_copy_buffer_to_image(
            device.device(),
            uploader.command_buffer(),
            uploader.staging_buffer().buffer,
            offset,
            image.image,
            &image.extent,
        );

        cmd_transition_image_layout(
            device.device(),
            uploader.command_buffer(),
            &[image.image],
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        images.push(image);
    }

    uploader.submit(queue)?;
    uploader.wait_until_finished()?;

    Ok(images)
}