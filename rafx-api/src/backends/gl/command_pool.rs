use crate::gl::{RafxCommandBufferGl, RafxDeviceContextGl, RafxQueueGl};
use crate::{RafxCommandBufferDef, RafxCommandPoolDef, RafxQueueType, RafxResult};
use rafx_base::trust_cell::TrustCell;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct CommandPoolGlStateInner {
    pub(crate) is_started: bool
}

#[derive(Clone, Debug)]
pub(crate) struct CommandPoolGlState {
    inner: Arc<TrustCell<CommandPoolGlStateInner>>
}

impl CommandPoolGlState {
    fn new() -> Self {
        let inner = CommandPoolGlStateInner {
            is_started: false
        };

        CommandPoolGlState {
            inner: Arc::new(TrustCell::new(inner))
        }
    }

    pub(crate) fn borrow(&self) -> rafx_base::trust_cell::Ref<CommandPoolGlStateInner> {
        self.inner.borrow()
    }

    pub(crate) fn borrow_mut(&self) -> rafx_base::trust_cell::RefMut<CommandPoolGlStateInner> {
        self.inner.borrow_mut()
    }
}

pub struct RafxCommandPoolGl {
    command_pool_state: CommandPoolGlState,
    queue: RafxQueueGl,
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

    pub(crate) fn command_pool_state(&self) -> &CommandPoolGlState {
        &self.command_pool_state
    }

    pub fn create_command_buffer(
        &self,
        command_buffer_def: &RafxCommandBufferDef,
    ) -> RafxResult<RafxCommandBufferGl> {
        RafxCommandBufferGl::new(self, command_buffer_def)
    }

    pub fn reset_command_pool(&self) -> RafxResult<()> {
        // Clear state
        let state = self.command_pool_state.borrow_mut();
        assert!(!state.is_started);

        Ok(())
    }

    pub fn new(
        queue: &RafxQueueGl,
        _command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolGl> {
        Ok(RafxCommandPoolGl {
            command_pool_state: CommandPoolGlState::new(),
            queue: queue.clone(),
        })
    }
}
