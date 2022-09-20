use crate::vulkan::{RafxCommandBufferVulkan, RafxDeviceContextVulkan, RafxQueueVulkan};
use crate::*;
use ash::vk;

pub struct RafxCommandPoolVulkan {
    device_context: RafxDeviceContextVulkan,
    vk_command_pool: vk::CommandPool,
    queue_type: RafxQueueType,
    queue_family_index: u32,
}

impl Drop for RafxCommandPoolVulkan {
    fn drop(&mut self) {
        unsafe {
            self.device_context
                .device()
                .destroy_command_pool(self.vk_command_pool, None);
        }
    }
}

impl RafxCommandPoolVulkan {
    pub fn device_context(&self) -> &RafxDeviceContextVulkan {
        &self.device_context
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.queue_type
    }

    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    pub fn vk_command_pool(&self) -> vk::CommandPool {
        self.vk_command_pool
    }

    pub fn create_command_buffer(
        &self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferVulkan> {
        RafxCommandBufferVulkan::new(&self, command_buffer_def)
    }

    #[profiling::function]
    pub fn reset_command_pool(&self) -> RafxResult<()> {
        unsafe {
            self.device_context
                .device()
                .reset_command_pool(self.vk_command_pool, vk::CommandPoolResetFlags::empty())?;
        }
        Ok(())
    }

    pub fn new(
        queue: &RafxQueueVulkan,
        command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolVulkan> {
        let queue_family_index = queue.queue().queue_family_index();
        log::trace!(
            "Creating command pool on queue family index {:?}",
            queue_family_index
        );

        let mut command_pool_create_flags = vk::CommandPoolCreateFlags::empty();
        if command_pool_def.transient {
            command_pool_create_flags |= vk::CommandPoolCreateFlags::TRANSIENT;
        }

        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(command_pool_create_flags)
            .queue_family_index(queue_family_index);

        let vk_command_pool = unsafe {
            queue
                .device_context()
                .device()
                .create_command_pool(&pool_create_info, None)?
        };

        Ok(RafxCommandPoolVulkan {
            device_context: queue.device_context().clone(),
            vk_command_pool,
            queue_type: queue.queue_type(),
            queue_family_index,
        })
    }
}
