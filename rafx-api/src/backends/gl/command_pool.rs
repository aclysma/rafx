use crate::gl::{RafxCommandBufferGl, RafxDeviceContextGl, RafxQueueGl};
use crate::{RafxCommandBufferDef, RafxCommandPoolDef, RafxQueueType, RafxResult};

pub struct RafxCommandPoolGl {
    queue: RafxQueueGl,
    //TODO: Tracking data for current state of the GPU?
}

impl RafxCommandPoolGl {
    pub fn device_context(&self) -> &RafxDeviceContextGl {
        self.queue.device_context()
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.queue.queue_type()
    }

    pub fn queue(&self) -> &RafxQueueGl {
        &self.queue
    }

    pub fn create_command_buffer(
        &self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferGl> {
        RafxCommandBufferGl::new(self, command_buffer_def)
    }

    pub fn reset_command_pool(&self) -> RafxResult<()> {
        unimplemented!();
        // do nothing
        Ok(())
    }

    pub fn new(
        queue: &RafxQueueGl,
        _command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolGl> {
        Ok(RafxCommandPoolGl {
            queue: queue.clone(),
        })
    }
}
