use crate::gl::{RafxCommandBufferGl, RafxDeviceContextGl, RafxQueueGl, GlPipelineInfo};
use crate::{RafxCommandBufferDef, RafxCommandPoolDef, RafxQueueType, RafxResult, RafxExtents2D};
use rafx_base::trust_cell::TrustCell;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct CommandPoolGlStateInner {
    pub(crate) is_started: bool,
    pub(crate) surface_size: Option<RafxExtents2D>,
    pub(crate) current_gl_pipeline_info: Option<Arc<GlPipelineInfo>>,
    pub(crate) stencil_reference_value: u32,

    // One per possible bound vertex buffer (could be 1 per attribute!)
    pub(crate) vertex_buffer_byte_offsets: Vec<u32>,
    pub(crate) index_buffer_byte_offset: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct CommandPoolGlState {
    inner: Arc<TrustCell<CommandPoolGlStateInner>>
}

impl CommandPoolGlState {
    fn new(device_context: &RafxDeviceContextGl) -> Self {
        let inner = CommandPoolGlStateInner {
            is_started: false,
            surface_size: None,
            current_gl_pipeline_info: None,
            stencil_reference_value: 0,
            vertex_buffer_byte_offsets: vec![0; device_context.device_info().max_vertex_attribute_count as usize],
            index_buffer_byte_offset: 0
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
            command_pool_state: CommandPoolGlState::new(queue.device_context()),
            queue: queue.clone(),
        })
    }
}
