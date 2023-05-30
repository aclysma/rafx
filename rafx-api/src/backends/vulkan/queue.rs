use super::internal::VkQueue;
use crate::vulkan::{
    RafxCommandBufferVulkan, RafxCommandPoolVulkan, RafxDeviceContextVulkan, RafxFenceVulkan,
    RafxSemaphoreVulkan, RafxSwapchainVulkan,
};
use crate::*;
use ash::vk;

#[derive(Clone, Debug)]
pub struct RafxQueueVulkan {
    device_context: RafxDeviceContextVulkan,
    queue: VkQueue,
    queue_type: RafxQueueType,
}

impl RafxQueueVulkan {
    pub fn queue_id(&self) -> u32 {
        (self.queue.queue_family_index() << 16) | (self.queue.queue_index())
    }

    pub fn queue(&self) -> &VkQueue {
        &self.queue
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.queue_type
    }

    pub fn device_context(&self) -> &RafxDeviceContextVulkan {
        &self.device_context
    }

    pub fn create_command_pool(
        &self,
        command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolVulkan> {
        RafxCommandPoolVulkan::new(&self, command_pool_def)
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueVulkan> {
        let queue = match queue_type {
            RafxQueueType::Graphics => device_context
                .queue_allocator()
                .allocate_graphics_queue(&device_context),
            RafxQueueType::Compute => device_context
                .queue_allocator()
                .allocate_compute_queue(&device_context),
            RafxQueueType::Transfer => device_context
                .queue_allocator()
                .allocate_transfer_queue(&device_context),
        }
        .ok_or_else(|| format!("All queues of type {:?} already allocated", queue_type))?;

        Ok(RafxQueueVulkan {
            device_context: device_context.clone(),
            queue,
            queue_type,
        })
    }

    pub fn wait_for_queue_idle(&self) -> RafxResult<()> {
        let queue = self.queue.queue().lock().unwrap();
        unsafe {
            self.queue
                .device_context()
                .device()
                .queue_wait_idle(*queue)?;
        }

        Ok(())
    }

    pub fn submit(
        &self,
        command_buffers: &[&RafxCommandBufferVulkan],
        wait_semaphores: &[&RafxSemaphoreVulkan],
        signal_semaphores: &[&RafxSemaphoreVulkan],
        signal_fence: Option<&RafxFenceVulkan>,
    ) -> RafxResult<()> {
        let mut command_buffer_list = Vec::with_capacity(command_buffers.len());
        for command_buffer in command_buffers {
            command_buffer_list.push(command_buffer.vk_command_buffer());
        }

        let mut wait_semaphore_list = Vec::with_capacity(wait_semaphores.len());
        let mut wait_dst_stage_mask = Vec::with_capacity(wait_semaphores.len());
        for wait_semaphore in wait_semaphores {
            // Don't wait on a semaphore that will never signal
            //TODO: Assert or fail here?
            if wait_semaphore.signal_available() {
                wait_semaphore_list.push(wait_semaphore.vk_semaphore());
                wait_dst_stage_mask.push(vk::PipelineStageFlags::ALL_COMMANDS);

                wait_semaphore.set_signal_available(false);
            }
        }

        let mut signal_semaphore_list = Vec::with_capacity(signal_semaphores.len());
        for signal_semaphore in signal_semaphores {
            // Don't signal a semaphore if something is already going to signal it
            //TODO: Assert or fail here?
            if !signal_semaphore.signal_available() {
                signal_semaphore_list.push(signal_semaphore.vk_semaphore());
                signal_semaphore.set_signal_available(true);
            }
        }

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphore_list)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .signal_semaphores(&signal_semaphore_list)
            .command_buffers(&command_buffer_list);

        let fence = signal_fence
            .map(|x| x.vk_fence())
            .unwrap_or(vk::Fence::null());
        unsafe {
            let queue = self.queue.queue().lock().unwrap();
            log::trace!(
                "submit {} command buffers to queue {:?}",
                command_buffer_list.len(),
                *queue
            );
            self.queue
                .device_context()
                .device()
                .queue_submit(*queue, &[*submit_info], fence)?;
        }

        if let Some(signal_fence) = signal_fence {
            signal_fence.set_submitted(true);
        }

        Ok(())
    }

    pub fn present(
        &self,
        swapchain: &RafxSwapchainVulkan,
        wait_semaphores: &[&RafxSemaphoreVulkan],
        image_index: u32,
    ) -> RafxResult<RafxPresentSuccessResult> {
        let mut wait_semaphore_list = Vec::with_capacity(wait_semaphores.len());
        for wait_semaphore in wait_semaphores {
            if wait_semaphore.signal_available() {
                wait_semaphore_list.push(wait_semaphore.vk_semaphore());
                wait_semaphore.set_signal_available(false);
            }
        }

        let swapchains = [swapchain.vk_swapchain()];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphore_list)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        //TODO: PresentInfoKHRBuilder::results() is only useful for presenting multiple swapchains -
        // presumably that's for multiwindow cases.

        let result = self.present_to_given_or_dedicated_queue(swapchain, &*present_info);

        match result {
            Ok(is_suboptimial) => {
                if is_suboptimial {
                    Ok(RafxPresentSuccessResult::SuccessSuboptimal)
                } else {
                    Ok(RafxPresentSuccessResult::Success)
                }
            }
            Err(e) => match e {
                RafxError::VkError(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    Ok(RafxPresentSuccessResult::DeviceReset)
                }
                e @ _ => Err(e),
            },
        }
    }

    // Make sure we always use the dedicated queue if it exists
    fn present_to_given_or_dedicated_queue(
        &self,
        swapchain: &RafxSwapchainVulkan,
        present_info: &vk::PresentInfoKHR,
    ) -> RafxResult<bool> {
        let is_suboptimal =
            if let Some(dedicated_present_queue) = swapchain.dedicated_present_queue() {
                // Because of the way we search for present-compatible queues, we don't necessarily have
                // the same underlying mutex in all instances of a dedicated present queue. So fallback
                // to a single global lock
                let _dedicated_present_lock = self
                    .device_context
                    .dedicated_present_queue_lock()
                    .lock()
                    .unwrap();
                unsafe {
                    log::trace!(
                        "present to dedicated present queue {:?}",
                        dedicated_present_queue
                    );
                    swapchain
                        .vk_swapchain_loader()
                        .queue_present(dedicated_present_queue, present_info)?
                }
            } else {
                let queue = self.queue.queue().lock().unwrap();
                log::trace!("present to dedicated present queue {:?}", *queue);
                unsafe {
                    swapchain
                        .vk_swapchain_loader()
                        .queue_present(*queue, &present_info)?
                }
            };

        Ok(is_suboptimal)
    }
}
