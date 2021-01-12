#[cfg(feature = "rafx-metal")]
use crate::metal::RafxQueueMetal;
use crate::vulkan::RafxQueueVulkan;
use crate::{
    RafxCommandBuffer, RafxCommandPool, RafxCommandPoolDef, RafxFence, RafxPresentSuccessResult,
    RafxQueueType, RafxResult, RafxSemaphore, RafxSwapchain,
};

#[derive(Clone, Debug)]
pub enum RafxQueue {
    Vk(RafxQueueVulkan),
    #[cfg(feature = "rafx-metal")]
    Metal(RafxQueueMetal),
}

impl RafxQueue {
    pub fn queue_id(&self) -> u32 {
        match self {
            RafxQueue::Vk(inner) => inner.queue_id(),
            #[cfg(feature = "rafx-metal")]
            RafxQueue::Metal(inner) => unimplemented!(),
        }
    }

    pub fn queue_type(&self) -> RafxQueueType {
        match self {
            RafxQueue::Vk(inner) => inner.queue_type(),
            #[cfg(feature = "rafx-metal")]
            RafxQueue::Metal(inner) => inner.queue_type(),
        }
    }

    pub fn create_command_pool(
        &self,
        command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPool> {
        Ok(match self {
            RafxQueue::Vk(inner) => {
                RafxCommandPool::Vk(inner.create_command_pool(command_pool_def)?)
            }
            #[cfg(feature = "rafx-metal")]
            RafxQueue::Metal(_inner) => unimplemented!(),
        })
    }

    pub fn submit(
        &self,
        command_buffers: &[&RafxCommandBuffer],
        wait_semaphores: &[&RafxSemaphore],
        signal_semaphores: &[&RafxSemaphore],
        signal_fence: Option<&RafxFence>,
    ) -> RafxResult<()> {
        match self {
            RafxQueue::Vk(inner) => {
                let command_buffers: Vec<_> = command_buffers
                    .iter()
                    .map(|x| x.vk_command_buffer().unwrap())
                    .collect();
                let wait_semaphores: Vec<_> = wait_semaphores
                    .iter()
                    .map(|x| x.vk_semaphore().unwrap())
                    .collect();
                let signal_semaphores: Vec<_> = signal_semaphores
                    .iter()
                    .map(|x| x.vk_semaphore().unwrap())
                    .collect();
                inner.submit(
                    &command_buffers,
                    &wait_semaphores,
                    &signal_semaphores,
                    signal_fence.map(|x| x.vk_fence().unwrap()),
                )
            }
            #[cfg(feature = "rafx-metal")]
            RafxQueue::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn present(
        &self,
        swapchain: &RafxSwapchain,
        wait_semaphores: &[&RafxSemaphore],
        image_index: u32,
    ) -> RafxResult<RafxPresentSuccessResult> {
        match self {
            RafxQueue::Vk(inner) => {
                let wait_semaphores: Vec<_> = wait_semaphores
                    .iter()
                    .map(|x| x.vk_semaphore().unwrap())
                    .collect();
                inner.present(
                    swapchain.vk_swapchain().unwrap(),
                    &wait_semaphores,
                    image_index,
                )
            }
            #[cfg(feature = "rafx-metal")]
            RafxQueue::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn wait_for_queue_idle(&self) -> RafxResult<()> {
        match self {
            RafxQueue::Vk(inner) => inner.wait_for_queue_idle(),
            #[cfg(feature = "rafx-metal")]
            RafxQueue::Metal(_inner) => unimplemented!(),
        }
    }

    pub fn vk_queue(&self) -> Option<&RafxQueueVulkan> {
        match self {
            RafxQueue::Vk(inner) => Some(inner),
            #[cfg(feature = "rafx-metal")]
            RafxQueue::Metal(_inner) => None,
        }
    }

    #[cfg(feature = "rafx-metal")]
    pub fn metal_queue(&self) -> Option<&RafxQueueMetal> {
        match self {
            RafxQueue::Vk(_inner) => None,
            RafxQueue::Metal(inner) => Some(inner),
        }
    }
}
