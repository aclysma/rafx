use crate::gl::{
    DescriptorSetArrayData, GlPipelineInfo, RafxCommandBufferGl, RafxDeviceContextGl, RafxQueueGl,
    RafxRootSignatureGl,
};
use crate::{
    RafxCommandBufferDef, RafxCommandPoolDef, RafxExtents2D, RafxQueueType, RafxResult,
    MAX_DESCRIPTOR_SET_LAYOUTS,
};
use rafx_base::trust_cell::TrustCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static NEXT_COMMAND_POOL_STATE_ID: AtomicU32 = AtomicU32::new(0);

pub(crate) struct BoundDescriptorSet {
    pub(crate) data: Arc<TrustCell<DescriptorSetArrayData>>,
    pub(crate) array_index: u32,
}

pub(crate) struct CommandPoolGlStateInner {
    pub(crate) id: u32,
    pub(crate) is_started: bool,
    pub(crate) surface_size: Option<RafxExtents2D>,
    pub(crate) current_gl_pipeline_info: Option<Arc<GlPipelineInfo>>,
    pub(crate) stencil_reference_value: u32,

    pub(crate) bound_descriptor_sets: [Option<BoundDescriptorSet>; MAX_DESCRIPTOR_SET_LAYOUTS],
    pub(crate) bound_descriptor_sets_root_signature: Option<RafxRootSignatureGl>,
    pub(crate) descriptor_sets_update_index: [u64; MAX_DESCRIPTOR_SET_LAYOUTS],

    // One per possible bound vertex buffer (could be 1 per attribute!)
    pub(crate) vertex_buffer_byte_offsets: Vec<u32>,
    pub(crate) index_buffer_byte_offset: u32,
}

impl PartialEq for CommandPoolGlStateInner {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.id == other.id
    }
}

impl std::fmt::Debug for CommandPoolGlStateInner {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("CommandPoolGlStateInner")
            .field("is_started", &self.is_started)
            .field("surface_size", &self.surface_size)
            .field("current_gl_pipeline_info", &self.current_gl_pipeline_info)
            .field("stencil_reference_value", &self.stencil_reference_value)
            .field(
                "vertex_buffer_byte_offsets",
                &self.vertex_buffer_byte_offsets,
            )
            .field("index_buffer_byte_offset", &self.index_buffer_byte_offset)
            .finish()
    }
}

impl CommandPoolGlStateInner {
    pub(crate) fn clear_bindings(&mut self) {
        for i in 0..MAX_DESCRIPTOR_SET_LAYOUTS {
            self.bound_descriptor_sets[i] = None;
            self.descriptor_sets_update_index[i] += 1;
            self.bound_descriptor_sets_root_signature = None;
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CommandPoolGlState {
    inner: Arc<TrustCell<CommandPoolGlStateInner>>,
}

impl CommandPoolGlState {
    fn new(device_context: &RafxDeviceContextGl) -> Self {
        let inner = CommandPoolGlStateInner {
            id: NEXT_COMMAND_POOL_STATE_ID.fetch_add(1, Ordering::Relaxed),
            is_started: false,
            surface_size: None,
            current_gl_pipeline_info: None,
            stencil_reference_value: 0,
            vertex_buffer_byte_offsets: vec![
                0;
                device_context.device_info().max_vertex_attribute_count
                    as usize
            ],
            index_buffer_byte_offset: 0,
            bound_descriptor_sets: Default::default(),
            bound_descriptor_sets_root_signature: None,
            descriptor_sets_update_index: Default::default(),
        };

        CommandPoolGlState {
            inner: Arc::new(TrustCell::new(inner)),
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
