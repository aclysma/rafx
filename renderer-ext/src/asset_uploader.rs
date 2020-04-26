type FrameIndex = u32;
type AssetUuid = u32;

trait AssetUploader {

    /// When called, assets that were unloaded during the given frame are destroyed
    fn on_gpu_frame_present_complete(&mut self, frame_index: FrameIndex);

    //
    // Calls from the asset system
    //

    /// Initiates uploading data for the given asset. When complete, load_op.complete() will be
    /// called
    fn update_asset(&mut self, uuid: AssetUuid, data: Vec<u8>, version: u32);

    /// Sets the given version as most recently committed, which means future calls to
    /// on_gpu_begin_read_for_frame will pin the frame to the committed data
    fn commit_asset_version(&mut self, version: u32);

    /// Queues resources to be released once the current frame ends (most recent index passed to
    /// on_gpu_begin_read_for_frame). Future frames will not have access to this resource at all.
    fn free(&mut self, uuid: AssetUuid, last_used_by_frame_index: FrameIndex);


    // //
    // // Fetches data
    // //
    //
    // /// Returns the GPU resource associated with the given frame.
    // ///
    // /// WARNING: Do not mix resources from before and after an asset loading tick.
    // fn get_resource(&self, uuid: AssetUuid) -> Vec<u8>;
}

use ash::vk;
use renderer_shell_vulkan::VkImage;
use renderer_shell_vulkan::VkBuffer;
use crate::renderpass::sprite::DecodedTexture;
use std::mem::ManuallyDrop;
use ash::prelude::*;
use fnv::FnvHashMap;
use ash::version::DeviceV1_0;

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


pub fn transition_image_layout(
    logical_device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    image: vk::Image,
    _format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> VkResult<()> {
    submit_single_use_command_buffer(logical_device, queue, command_pool, |command_buffer| {
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

        let barrier_info = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(*subresource_range)
            .src_access_mask(sync_info.src_access_mask)
            .dst_access_mask(sync_info.dst_access_mask);

        unsafe {
            logical_device.cmd_pipeline_barrier(
                command_buffer,
                sync_info.src_stage,
                sync_info.dst_stage,
                vk::DependencyFlags::BY_REGION,
                &[],
                &[],
                &[*barrier_info],
            ); //TODO: Can remove build() by using *?
        }
    })
}

pub fn copy_buffer_to_image(
    logical_device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    buffer: vk::Buffer,
    image: vk::Image,
    extent: &vk::Extent3D,
) -> VkResult<()> {
    submit_single_use_command_buffer(logical_device, queue, command_pool, |command_buffer| {
        let image_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1);

        let image_copy = vk::BufferImageCopy::builder()
            .buffer_offset(0)
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
    })
}
/*
pub fn load_images(
    logical_device: &ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
    decoded_texture: &[DecodedTexture],
) -> VkResult<ManuallyDrop<VkImage>> {
    let extent = vk::Extent3D {
        width: decoded_texture.width,
        height: decoded_texture.height,
        depth: 1,
    };

    let mut staging_buffer = VkBuffer::new(
        logical_device,
        device_memory_properties,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        decoded_texture.data.len() as u64,
    )?;

    staging_buffer.write_to_host_visible_buffer(&decoded_texture.data)?;

    let image = VkImage::new(
        logical_device,
        device_memory_properties,
        extent,
        vk::Format::R8G8B8A8_UNORM,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    transition_image_layout(
        logical_device,
        queue,
        command_pool,
        image.image,
        vk::Format::R8G8B8A8_UNORM,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    )?;

    copy_buffer_to_image(
        logical_device,
        queue,
        command_pool,
        staging_buffer.buffer,
        image.image,
        &image.extent,
    )?;

    transition_image_layout(
        logical_device,
        queue,
        command_pool,
        image.image,
        vk::Format::R8G8B8A8_UNORM,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )?;

    Ok(ManuallyDrop::new(image))
}
*/



type TextureHandle = u32;

// Assets that need to be copied to a staging buffer
struct PendingUpload {
    // UUID
    uuid: AssetUuid,
    data: Vec<u8>
}

// Assets that are in a staging buffer and have a command issued to copy into a command buffer
struct InProgressUpload {
    // UUIDs included in the command buffer
    // Fence that can be checked
    // staging buffer
    // device buffer
    uuid: AssetUuid,
    data: Vec<u8>
}

// Assets that have been uploaded
struct CompletedUpload {
    // device buffer
    uuid: AssetUuid,
    data: TextureHandle
}

#[derive(Default)]
struct TextureAssetUploader {
    pending_uploads: Vec<PendingUpload>,
    in_progress_uploads: Vec<InProgressUpload>,
    completed_uploads: Vec<CompletedUpload>,
    pending_removes: FnvHashMap<FrameIndex, Vec<AssetUuid>>,

    assets: FnvHashMap<AssetUuid, TextureHandle>,

    //current_frame: u32,
}

impl TextureAssetUploader {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(&mut self) {
        for pending_upload in self.pending_uploads.drain(..) {
            self.in_progress_uploads.push(InProgressUpload {
                uuid: pending_upload.uuid,
                data: pending_upload.data
            });
        }

        for in_progress_upload in self.in_progress_uploads.drain(..) {
            self.completed_uploads.push(CompletedUpload {
                uuid: in_progress_upload.uuid,
                data: 0
            });
        }
    }

    /// Returns the GPU resource associated with the given frame.
    ///
    /// WARNING: Do not mix resources from before and after an asset loading tick.
    fn get_resource(&self, uuid: AssetUuid) -> Option<TextureHandle> {
        // Try to fetch it from assets
        self.assets.get(&uuid).map(|x| *x)
    }
}

impl AssetUploader for TextureAssetUploader {
    fn on_gpu_frame_present_complete(&mut self, frame_index: FrameIndex) {
        // Drop everything in pending_unloads under frame_index out of the asset hashmap
        let assets_to_remove = self.pending_removes.get(&frame_index);
        if let Some(assets_to_remove) = assets_to_remove {
            for asset in assets_to_remove {
                self.assets.remove(asset);
            }
        }

        self.pending_removes.remove(&frame_index);
    }

    fn update_asset(&mut self, uuid: AssetUuid, data: Vec<u8>, version: u32) {
        // Push the data into pending_uploads.. either kick off a task to do the upload or
        // wait until later to kick it off as a batch

        self.pending_uploads.push(PendingUpload {
            uuid,
            data
        });
    }

    /// Sets the given version as most recently committed, which means future calls to
    /// on_gpu_begin_read_for_frame will pin the frame to the committed data
    fn commit_asset_version(&mut self, version: u32) {
        // Copy completed uploads into the assets hash map
        for completed_upload in self.completed_uploads.drain(..) {
            self.assets.insert(completed_upload.uuid, completed_upload.data);
        }
    }

    /// Queues resources to be released once the current frame ends (most recent index passed to
    /// on_gpu_begin_read_for_frame). Future frames will not have access to this resource at all.
    fn free(&mut self, uuid: AssetUuid, last_used_by_frame_index: FrameIndex) {
        // Push the asset into pending_unloads
        self.pending_removes.entry(last_used_by_frame_index).or_default().push(uuid);
    }
}
