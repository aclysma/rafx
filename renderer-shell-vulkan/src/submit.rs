
use ash::vk;
use std::sync::{mpsc, Mutex};
use ash::prelude::VkResult;
use ash::version::DeviceV1_0;
use ash::vk::SubmitInfo;
use crate::{VkDevice, VkDeviceContext};
use std::sync::mpsc::{Receiver, Sender};

/*
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
    pub semaphore_waits_dst_stage_mask: Vec<vk::PipelineStageFlags>,
    pub semaphore_signals: Vec<vk::Semaphore>,

    pub command_pools_reset_on_finish: Vec<vk::CommandPool>,
}

struct InFlightSubmit {
    pending_command_buffers: Vec<Box<PendingCommandBuffer>>,
    submit_finished_fence: vk::Fence
}

pub struct VkSubmitQueueInner {
    // Would be a DeviceContext except the DeviceContext can own these
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
    fence_pool: Vec<vk::Fence>,

    pending_command_buffers_rx: Receiver<Box<PendingCommandBuffer>>,

    pending_command_buffers: Vec<Box<PendingCommandBuffer>>,
    in_flight_submits: Vec<Box<InFlightSubmit>>,
}

pub struct VkSubmitQueue {
    inner: Mutex<VkSubmitQueueInner>,
    pending_command_buffer_tx: Sender<Box<PendingCommandBuffer>>,
}

impl VkSubmitQueue {
    pub fn new(
        device: ash::Device,
        queue: vk::Queue,
        queue_family_index: u32
    ) -> Self {

        let (pending_command_buffer_tx, pending_command_buffers_rx) = mpsc::channel();

        let inner = VkSubmitQueueInner {
            device,
            queue,
            queue_family_index,
            fence_pool: Default::default(), //TODO: Preallocate?
            pending_command_buffers_rx,
            pending_command_buffers: Default::default(),
            in_flight_submits: Default::default()
        };

        VkSubmitQueue {
            inner: Mutex::new(inner),
            pending_command_buffer_tx
        }
    }

    pub fn push(&self, pending_command_buffer: Box<PendingCommandBuffer>) {
        self.pending_command_buffer_tx.send(pending_command_buffer);
    }

    pub fn submit(
        &self,
        submit_wait_semaphores: &[vk::Semaphore],
        submit_wait_dst_stage_mask: &[vk::PipelineStageFlags],
        submit_signal_semaphores: &[vk::Semaphore],
        submit_signal_fences: &[vk::Fence]
    ) -> VkResult<()> {
        let mut inner = self.inner.lock().unwrap();

        println!("*** SUBMIT QUEUE");
        // Grab all the command buffers to submit
        let mut pending_command_buffers = vec![];
        std::mem::swap(&mut pending_command_buffers, &mut inner.pending_command_buffers);

        // Pull any new ones out of the queue to push into the list
        for pending_command_buffer in inner.pending_command_buffers_rx.recv_timeout(std::time::Duration::default()) {
            pending_command_buffers.push(pending_command_buffer);
        }

        let mut submits = Vec::with_capacity(inner.pending_command_buffers.len());

        // // If there are no
        // if pending_command_buffers.is_empty() {
        //     // We can't skip if there are semaphores because there is nothing to attach them to
        //     assert!(submit_signal_semaphores.is_empty() && pending_command_buffers.is_empty());
        //     return Ok(());
        // }

        if pending_command_buffers.is_empty() && submit_signal_semaphores.is_empty() && pending_command_buffers.is_empty() {
            return Ok(())
        }

        if let Some(first) = pending_command_buffers.first_mut() {
            for wait_semaphore in submit_wait_semaphores {
                println!("Add wait semaphore {:?}", wait_semaphore);
                first.semaphore_waits.push(*wait_semaphore);
            }

            for dst_stage_mask in submit_wait_dst_stage_mask {
                println!("Add dst stage mask {:?}", dst_stage_mask);
                first.semaphore_waits_dst_stage_mask.push(*dst_stage_mask);
            }
        }

        if let Some(last) = pending_command_buffers.last_mut() {
            for signal_semaphore in submit_signal_semaphores {
                println!("Add signal semaphore {:?}", signal_semaphore);
                last.semaphore_signals.push(*signal_semaphore);
            }
        }

        // // If there is a wait semaphore, add a submit info for it
        // if !submit_wait_semaphores.is_empty() {
        //     println!("Add wait semaphore {:?} dst_stage_mask {:?}", submit_wait_semaphores, submit_wait_dst_stage_mask);
        //     let submit_info = vk::SubmitInfo::builder()
        //         .wait_semaphores(submit_wait_semaphores)
        //         .wait_dst_stage_mask(submit_wait_dst_stage_mask)
        //         .build();
        //
        //     submits.push(submit_info);
        // }

        // Generate a submit info for each pending command buffer
        for pending_command_buffer in &pending_command_buffers {
            println!("  wait semaphores {:?}", pending_command_buffer.semaphore_waits);
            println!("  signal semaphores {:?}", pending_command_buffer.semaphore_signals);


            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&pending_command_buffer.command_buffers)
                .signal_semaphores(&pending_command_buffer.semaphore_waits)
                .wait_dst_stage_mask(&pending_command_buffer.semaphore_waits_dst_stage_mask)
                .wait_semaphores(&pending_command_buffer.semaphore_signals)
                .build();

            submits.push(submit_info);
        }

        // // If there is a signal semaphore, add a submit info for it
        // if !submit_signal_semaphores.is_empty() {
        //     println!("Add signal semaphore {:?}", submit_signal_semaphores);
        //     let submit_info = vk::SubmitInfo::builder()
        //         .signal_semaphores(submit_signal_semaphores)
        //         .build();
        //
        //     submits.push(submit_info);
        // }

        let mut fences = Vec::with_capacity(submit_signal_fences.len() + 1);
        for fence in submit_signal_fences {
            fences.push(*fence);
        }

        // Allocate and push a fence if the pool is empty
        if inner.fence_pool.is_empty() {
            let fence_create_info = vk::FenceCreateInfo::builder().build();
            unsafe {
                let fence = inner.device.create_fence(&fence_create_info, None)?;
                inner.fence_pool.push(fence);
            }
        }

        // Submit the command buffers and add an entry to the in-flight list
        //let submit_finished_fence = inner.fence_pool.pop().unwrap();
        //fences.push(submit_finished_fence);
        let submit_finished_fence = submit_signal_fences[0];
        unsafe {
            //println!("{:#?}", submits);
            inner.device.queue_submit(inner.queue, &submits, submit_finished_fence)?;
            inner.in_flight_submits.push(Box::new(InFlightSubmit {
                pending_command_buffers,
                submit_finished_fence
            }));
        }

        Ok(())
    }

    pub fn update(&self) -> VkResult<()> {
        let mut inner = self.inner.lock().unwrap();

        // Take all the in-flight submits, we'll put back any that are still in-flight
        let mut in_flight_submits = Vec::with_capacity(inner.in_flight_submits.len());
        std::mem::swap(&mut in_flight_submits, &mut inner.in_flight_submits);

        for in_flight_submit in in_flight_submits {
            let submit_finished = unsafe {
                inner.device.get_fence_status(in_flight_submit.submit_finished_fence)?
            };

            if submit_finished {
                unsafe {
                    // Do other on-finish work
                    for finished_command_buffer in in_flight_submit.pending_command_buffers {
                        for command_pool in finished_command_buffer.command_pools_reset_on_finish {
                            inner.device.reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty());
                        }
                    }

                    // Return the submit finished fence to the pool
                    inner.device.reset_fences(&[in_flight_submit.submit_finished_fence])?;
                    inner.fence_pool.push(in_flight_submit.submit_finished_fence);
                }
            } else {
                // It's not finished yet, put it back in the in-flight list
                inner.in_flight_submits.push(in_flight_submit);
            }
        }

        Ok(())
    }
}
*/