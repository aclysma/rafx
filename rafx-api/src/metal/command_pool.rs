use crate::metal::{RafxCommandBufferMetal, RafxDeviceContextMetal, RafxQueueMetal};
use crate::{RafxCommandBufferDef, RafxCommandPoolDef, RafxQueueType, RafxResult};

pub struct RafxCommandPoolMetal {
    queue: RafxQueueMetal,
}

impl RafxCommandPoolMetal {
    pub fn device_context(&self) -> &RafxDeviceContextMetal {
        self.queue.device_context()
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.queue.queue_type()
    }

    pub fn queue(&self) -> &RafxQueueMetal {
        &self.queue
    }

    pub fn create_command_buffer(
        &self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferMetal> {
        RafxCommandBufferMetal::new(self, command_buffer_def)
    }

    pub fn reset_command_pool(&self) -> RafxResult<()> {
        // do nothing
        Ok(())
    }

    pub fn new(
        queue: &RafxQueueMetal,
        _command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolMetal> {
        Ok(RafxCommandPoolMetal {
            queue: queue.clone(),
        })
    }
}
