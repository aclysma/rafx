
use ash::vk;
use std::sync::mpsc;
use renderer_shell_vulkan::{VkUpload, VkDevice, VkDeviceContext};
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk::SubmitInfo;

//TODO: CommmandBufferPool may need a custom strategy, one per thread/frame in flight

// Other things we could include here
// - fence to signal on complete
// - command buffers to submit
// - command pool to destroy on complete
// - event to destroy on complete
// - semaphore to wait for
// - semaphore to signal on complete
// - semaphore to reset on complete
// - semaphore to destroy on complete
// - fence to wait for
// - fence to signal on complete
// - fence to reset on complete
// - fence to destroy on complete
struct PendingCommandBuffer {
    command_buffers: Vec<vk::CommandBuffer>,

    semaphore_waits: Vec<vk::Semaphore>,
    semaphore_signals: Vec<vk::Semaphore>,

    command_pools_reset_on_finish: Vec<vk::CommandPool>,
}

struct InFlightSubmit {
    pending_command_buffers: Vec<PendingCommandBuffer>,
    submit_finished_fence: vk::Fence
}

struct SubmitQueue {
    device_context: VkDeviceContext,
    queue: vk::Queue,
    queue_family_index: u32,
    fence_pool: Vec<vk::Fence>,

    pending_command_buffers: Vec<PendingCommandBuffer>,
    in_flight_submits: Vec<Box<InFlightSubmit>>,
}

impl SubmitQueue {
    pub fn new(
        device: &VkDevice,
        queue: vk::Queue,
        queue_family_index: u32
    ) -> Self {
        SubmitQueue {
            device_context: device.context.clone(),
            queue,
            queue_family_index,
            fence_pool: Default::default(), //TODO: Preallocate?
            pending_command_buffers: Default::default(),
            in_flight_submits: Default::default()
        }
    }

    pub fn submit(&mut self) -> VkResult<()> {
        // Grab all the command buffers to submit
        let mut submits = Vec::with_capacity(self.pending_command_buffers.len());
        let mut pending_command_buffers = vec![];
        std::mem::swap(&mut pending_command_buffers, &mut self.pending_command_buffers);

        // Generate a submit info for each pending command buffer
        for pending_command_buffer in &pending_command_buffers {
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&pending_command_buffer.command_buffers)
                .signal_semaphores(&pending_command_buffer.semaphore_waits)
                .wait_semaphores(&pending_command_buffer.semaphore_signals)
                .build();

            submits.push(submit_info);
        }

        // Allocate and push a fence if the pool is empty
        if self.fence_pool.is_empty() {
            let fence_create_info = vk::FenceCreateInfo::builder().build();
            unsafe {
                self.fence_pool.push(self.device_context.device().create_fence(&fence_create_info, None)?);
            }
        }

        // Submit the command buffers and add an entry to the in-flight list
        let submit_finished_fence = self.fence_pool.pop().unwrap();
        unsafe {
            self.device_context.device().queue_submit(self.queue, &submits, submit_finished_fence)?;
            self.in_flight_submits.push(Box::new(InFlightSubmit {
                pending_command_buffers,
                submit_finished_fence
            }));
        }

        Ok(())
    }

    pub fn update(&mut self) -> VkResult<()> {
        // Take all the in-flight submits, we'll put back any that are still in-flight
        let mut in_flight_submits = Vec::with_capacity(self.in_flight_submits.len());
        std::mem::swap(&mut in_flight_submits, &mut self.in_flight_submits);

        for in_flight_submit in in_flight_submits {
            let submit_finished = unsafe {
                self.device_context.device().get_fence_status(in_flight_submit.submit_finished_fence)?
            };

            if submit_finished {
                unsafe {
                    // Do other on-finish work
                    for finished_command_buffer in in_flight_submit.pending_command_buffers {
                        for command_pool in finished_command_buffer.command_pools_reset_on_finish {
                            self.device_context.device().reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty());
                        }
                    }

                    // Return the submit finished fence to the pool
                    self.device_context.device().reset_fences(&[in_flight_submit.submit_finished_fence])?;
                    self.fence_pool.push(in_flight_submit.submit_finished_fence);
                }
            } else {
                // It's not finished yet, put it back in the in-flight list
                self.in_flight_submits.push(in_flight_submit);
            }
        }

        Ok(())
    }
}

//Do Draw:
// - Check frame in flight limit (by checking a ring buffer of fences)
// - acquire image, which will signal image_available
// - grab all pending command buffers
//   * They will wait for image_available semaphor
//   * They will signal render_finished and a frame in flight fence
// - present image, which will wait for render_finished semaphor
//
// This can be implemented with
// - submit_queue.submit(wait_semaphors, signal_semaphors,





// struct PendingCommandBuffer {
//     command_buffer: Vec<vk::CommandBuffer>,
//
//
// }
//
// struct QueueManager {
//     queue: vk::Queue,
//     queued_command_buffers: Vec<vk::CommandBuffer>,
//     fences_to_signal: Vec<vk::Fence>
// }


// struct VkTransferUploader {
//     uploader: VkUploader,
//
//     transfer_queue_family_index: u32,
//     transfer_command_pool: vk::CommandPool,
//     transfer_command_buffer: vk::CommandBuffer,
// }
//
// impl VkTransferUploader {
//     pub fn new(
//         device: &VkDevice,
//         graphics_queue_family_index: u32,
//         size: u64,
//         transfer_queue_family_index: u32,
//     ) -> Self {
//         let queue = device.queues.graphics_queue;
//
//         let uploader = VkUploader::new(deice, graphics_queue_family_index, size);
//
//         //
//         // Command Buffers
//         //
//         let command_pool =
//             Self::create_command_pool(device.device(), queue_family_index)?;
//
//         let command_buffer = Self::create_command_buffer(device.device(), &command_pool)?;
//         Self::begin_command_buffer(device.device(), command_buffer)?;
//     }
//
//     fn create_command_pool(
//         logical_device: &ash::Device,
//         queue_family_index: u32,
//     ) -> VkResult<vk::CommandPool> {
//         //TODO: Consider a separate transfer queue
//         log::info!(
//             "Creating command pool with queue family index {}",
//             queue_family_index
//         );
//         let pool_create_info = vk::CommandPoolCreateInfo::builder()
//             .flags(
//                 vk::CommandPoolCreateFlags::TRANSIENT
//                     | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
//             )
//             .queue_family_index(queue_family_index);
//
//         unsafe { logical_device.create_command_pool(&pool_create_info, None) }
//     }
//
//     fn create_command_buffer(
//         logical_device: &ash::Device,
//         command_pool: &vk::CommandPool,
//     ) -> VkResult<vk::CommandBuffer> {
//         let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
//             .command_buffer_count(1)
//             .command_pool(*command_pool)
//             .level(vk::CommandBufferLevel::PRIMARY);
//
//         unsafe {
//             Ok(logical_device.allocate_command_buffers(&command_buffer_allocate_info)?[0])
//         }
//     }
//
// }