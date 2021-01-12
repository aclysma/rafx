use crate::{RafxQueueType, RafxResult};

#[derive(Clone, Debug)]
pub struct RafxQueueMetal {
    command_queue: metal::CommandQueue,
    queue_type: RafxQueueType,
}

unsafe impl Send for RafxQueueMetal {}
unsafe impl Sync for RafxQueueMetal {}

impl RafxQueueMetal {
    pub fn queue_type(&self) -> RafxQueueType {
        self.queue_type
    }

    pub fn new(
        device: &metal::Device,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueMetal> {
        let command_queue = device.new_command_queue();
        Ok(RafxQueueMetal {
            command_queue,
            queue_type,
        })
    }

    pub fn command_queue(&self) -> &metal::CommandQueue {
        &self.command_queue
    }
}
