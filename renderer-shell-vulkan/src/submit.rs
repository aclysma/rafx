
use ash::vk;
use std::sync::mpsc;
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk::SubmitInfo;
use crate::{VkDevice, VkDeviceContext};
use std::sync::mpsc::{Receiver, Sender};

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
pub struct PendingCommandBuffer {
    pub command_buffers: Vec<vk::CommandBuffer>,

    pub semaphore_waits: Vec<vk::Semaphore>,
    pub semaphore_signals: Vec<vk::Semaphore>,

    pub command_pools_reset_on_finish: Vec<vk::CommandPool>,
}

struct InFlightSubmit {
    pending_command_buffers: Vec<Box<PendingCommandBuffer>>,
    submit_finished_fence: vk::Fence
}

pub struct VkSubmitQueue {
    // Would be a DeviceContext except the DeviceContext can own these
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
    fence_pool: Vec<vk::Fence>,

    pending_command_buffer_tx: Sender<Box<PendingCommandBuffer>>,
    pending_command_buffers_rx: Receiver<Box<PendingCommandBuffer>>,

    pending_command_buffers: Vec<Box<PendingCommandBuffer>>,
    in_flight_submits: Vec<Box<InFlightSubmit>>,
}

impl VkSubmitQueue {
    pub fn new(
        device: ash::Device,
        queue: vk::Queue,
        queue_family_index: u32
    ) -> Self {

        let (pending_command_buffer_tx, pending_command_buffers_rx) = mpsc::channel();

        VkSubmitQueue {
            device,
            queue,
            queue_family_index,
            fence_pool: Default::default(), //TODO: Preallocate?
            pending_command_buffer_tx,
            pending_command_buffers_rx,
            pending_command_buffers: Default::default(),
            in_flight_submits: Default::default()
        }
    }

    pub fn push(&self, pending_command_buffer: Box<PendingCommandBuffer>) {
        self.pending_command_buffer_tx.send(pending_command_buffer);
    }

    pub fn submit(
        &mut self,
        submit_wait_semaphores: &[vk::Semaphore],
        submit_signal_semaphores: &[vk::Semaphore]
    ) -> VkResult<()> {
        // Grab all the command buffers to submit
        let mut submits = Vec::with_capacity(self.pending_command_buffers.len());
        let mut pending_command_buffers = vec![];
        std::mem::swap(&mut pending_command_buffers, &mut self.pending_command_buffers);

        for pending_command_buffer in self.pending_command_buffers_rx.recv() {
            pending_command_buffers.push(pending_command_buffer);
        }

        if let Some(first) = pending_command_buffers.first_mut() {
            for wait_semaphore in submit_wait_semaphores {
                first.semaphore_waits.push(*wait_semaphore);
            }
        }

        if let Some(last) = pending_command_buffers.last_mut() {
            for signal_semaphore in submit_signal_semaphores {
                last.semaphore_waits.push(*signal_semaphore);
            }
        }

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
                self.fence_pool.push(self.device.create_fence(&fence_create_info, None)?);
            }
        }

        // Submit the command buffers and add an entry to the in-flight list
        let submit_finished_fence = self.fence_pool.pop().unwrap();
        unsafe {
            self.device.queue_submit(self.queue, &submits, submit_finished_fence)?;
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
                self.device.get_fence_status(in_flight_submit.submit_finished_fence)?
            };

            if submit_finished {
                unsafe {
                    // Do other on-finish work
                    for finished_command_buffer in in_flight_submit.pending_command_buffers {
                        for command_pool in finished_command_buffer.command_pools_reset_on_finish {
                            self.device.reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty());
                        }
                    }

                    // Return the submit finished fence to the pool
                    self.device.reset_fences(&[in_flight_submit.submit_finished_fence])?;
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
