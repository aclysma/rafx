use ash::vk;
use ash::prelude::VkResult;

use ash::version::DeviceV1_0;
use crate::{VkDevice, VkQueueFamilyIndices, VkBuffer, VkDeviceContext};
use std::mem::ManuallyDrop;
use std::os::raw::c_void;
use ash::vk::MappedMemoryRange;
use std::ops::Deref;

// Based on UploadHeap in cauldron
// (https://github.com/GPUOpen-LibrariesAndSDKs/Cauldron/blob/5acc12602c55e469cc1f9181967dbcb122f8e6c7/src/VK/base/UploadHeap.h)

#[derive(PartialEq)]
enum UploaderState {
    Writable,
    SentToGpu
}

/// This is a convenience class that allows accumulating writes into a staging buffer and commands
/// to execute on the staging buffer. This allows for batching uploading resources.
pub struct VkUploader {
    device_context: VkDeviceContext,

    queue_family_index: u32,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    buffer: ManuallyDrop<VkBuffer>,

    fence: vk::Fence,

    bytes_written_to_buffer: u64,
    state: UploaderState,

    buffer_begin: *mut u8,
    buffer_end: *mut u8,
    buffer_write_pointer: *mut u8
}

impl VkUploader {
    pub fn new(
        device: &VkDevice,
        queue_family_index: u32,
        size: u64
    ) -> VkResult<Self> {
        let queue = device.queues.graphics_queue;

        //
        // Command Buffers
        //
        let command_pool =
            Self::create_command_pool(device.device(), queue_family_index)?;

        let command_buffer = Self::create_command_buffer(device.device(), &command_pool)?;
        Self::begin_command_buffer(device.device(), command_buffer)?;

        let buffer = ManuallyDrop::new(VkBuffer::new(
            &device.context,
            vk_mem::MemoryUsage::CpuOnly,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            size
        )?);

        let (buffer_begin, buffer_end, buffer_write_pointer) = unsafe {
            //TODO: Better way of handling allocator errors
            let buffer_begin = device.allocator().map_memory(
                &buffer.allocation
            ).map_err(|_| vk::Result::ERROR_MEMORY_MAP_FAILED)? as *mut u8;

            let buffer_end = buffer_begin.add(buffer.size() as usize);
            let buffer_write_pointer = buffer_begin;

            (buffer_begin, buffer_end, buffer_write_pointer)
        };

        let fence = Self::create_fence(device.device())?;

        let mut uploader = VkUploader {
            device_context: device.context.clone(),
            queue_family_index,
            command_pool,
            command_buffer,
            buffer,
            fence,
            bytes_written_to_buffer: 0,
            state: UploaderState::Writable,
            buffer_begin,
            buffer_end,
            buffer_write_pointer
        };

        Ok(uploader)
    }

    fn create_command_pool(
        logical_device: &ash::Device,
        queue_family_index: u32,
    ) -> VkResult<vk::CommandPool> {
        //TODO: Consider a separate transfer queue
        log::info!(
            "Creating command pool with queue family index {}",
            queue_family_index
        );
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(
                vk::CommandPoolCreateFlags::TRANSIENT
                    | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            )
            .queue_family_index(queue_family_index);

        unsafe { logical_device.create_command_pool(&pool_create_info, None) }
    }

    fn create_command_buffer(
        logical_device: &ash::Device,
        command_pool: &vk::CommandPool,
    ) -> VkResult<vk::CommandBuffer> {
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(*command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        unsafe {
            Ok(logical_device.allocate_command_buffers(&command_buffer_allocate_info)?[0])
        }
    }

    fn create_fence(
        logical_device: &ash::Device,
    ) -> VkResult<vk::Fence> {
        let fence_create_info =
            vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::empty());

        unsafe {
            Ok(logical_device.create_fence(&fence_create_info, None)?)
        }
    }

    fn begin_command_buffer(
        logical_device: &ash::Device,
        command_buffer: vk::CommandBuffer
    ) -> VkResult<()> {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::empty());
        unsafe {
            logical_device.begin_command_buffer(command_buffer, &command_buffer_begin_info)
        }
    }

    fn reset_to_writable_state(
        &mut self
    ) -> VkResult<()> {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::empty());

        unsafe {
            Self::begin_command_buffer(self.device_context.device(), self.command_buffer);
            self.device_context.device().reset_fences(&[self.fence])?;
            self.buffer_write_pointer = self.buffer_begin;
            self.state = UploaderState::Writable;
        }

        Ok(())
    }

    pub fn push(&mut self, data: &[u8], required_alignment: usize) -> VkResult<vk::DeviceSize> {
        log::debug!("Pushing {} bytes into uploader", data.len());

        if self.state == UploaderState::Writable {
            unsafe {
                // Figure out the span of memory we will write over
                let align_offset = self.buffer_write_pointer as usize % required_alignment;
                let write_begin_ptr = self.buffer_write_pointer.add(align_offset);
                let write_end_ptr = write_begin_ptr.add(data.len());

                // If the span walks past the end of the buffer, fail
                if write_end_ptr > self.buffer_end {
                    return Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY);
                }

                std::ptr::copy_nonoverlapping(data.as_ptr(), write_begin_ptr, data.len());
                self.buffer_write_pointer = write_end_ptr;

                Ok(write_begin_ptr as vk::DeviceSize - self.buffer_begin as vk::DeviceSize)
            }
        } else {
            Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)
        }
    }

    pub fn command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffer
    }

    pub fn staging_buffer(&self) -> &VkBuffer {
        &self.buffer
    }

    pub fn submit(&mut self, queue: vk::Queue) -> VkResult<()> {
        if self.state == UploaderState::Writable {
            unsafe {
                self.device_context.device().end_command_buffer(self.command_buffer)?;
            }

            //TODO: Submit and wait for fence

            let submit = vk::SubmitInfo::builder()
                .command_buffers(&[self.command_buffer])
                .build();

            unsafe {
                self.device_context.device().queue_submit(queue, &[submit], self.fence)?;
                self.state = UploaderState::SentToGpu;
            }
        }

        Ok(())
    }

    pub fn wait_until_finished(&mut self) -> VkResult<()> {
        if self.state == UploaderState::SentToGpu {
            unsafe {
                self.device_context.device().wait_for_fences(&[self.fence], true, std::u64::MAX)?;
                self.reset_to_writable_state();
            }
        }

        Ok(())
    }

    // pub fn update(&mut self) -> VkResult<()> {
    //     if self.state == UploaderState::SentToGpu {
    //         unsafe {
    //             let submit_complete = self.device_context.device().get_fence_status(self.fence)?;
    //
    //             if submit_complete {
    //                 self.reset_to_writable_state();
    //             }
    //         }
    //     }
    //
    //     Ok(())
    // }
}

impl Drop for VkUploader {
    fn drop(&mut self) {
        log::debug!("destroying VkUploader");

        unsafe {
            self.device_context.allocator().unmap_memory(&self.buffer.allocation);
            ManuallyDrop::drop(&mut self.buffer);
            self.device_context.device().destroy_command_pool(self.command_pool, None);
            self.device_context.device().destroy_fence(self.fence, None);
        }

        log::debug!("destroyed VkSpriteRenderPass");
    }
}
