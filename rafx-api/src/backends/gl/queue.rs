use crate::gl::{RafxCommandBufferGl, RafxCommandPoolGl, RafxDeviceContextGl, RafxFenceGl, RafxSemaphoreGl, RafxSwapchainGl, VertexArrayObjectId};
use crate::{RafxCommandPoolDef, RafxPresentSuccessResult, RafxQueueType, RafxResult};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

static NEXT_QUEUE_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
pub struct RafxQueueGlInner {
    device_context: RafxDeviceContextGl,
    queue_type: RafxQueueType,
    queue_id: u32,

    // We use one global VAO and rebind it when we bind vertex buffers. It is bound/unbound when
    // a command buffer begins/ends
    vertex_array_object: VertexArrayObjectId,
}

impl Drop for RafxQueueGlInner {
    fn drop(&mut self) {
        self.device_context.gl_context().gl_destroy_vertex_array(self.vertex_array_object);
    }
}

#[derive(Clone, Debug)]
pub struct RafxQueueGl {
    inner: Arc<RafxQueueGlInner>,
}

impl RafxQueueGl {
    pub fn queue_id(&self) -> u32 {
        self.inner.queue_id
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.inner.queue_type
    }

    pub fn device_context(&self) -> &RafxDeviceContextGl {
        &self.inner.device_context
    }

    pub(crate) fn gl_vertex_buffer_array_object(&self) -> VertexArrayObjectId {
        self.inner.vertex_array_object
    }

    pub fn create_command_pool(
        &self,
        command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolGl> {
        RafxCommandPoolGl::new(&self, command_pool_def)
    }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueGl> {
        let vertex_array_object = device_context.gl_context().gl_create_vertex_array()?;

        let queue_id = NEXT_QUEUE_ID.fetch_add(1, Ordering::Relaxed);
        let inner = RafxQueueGlInner {
            device_context: device_context.clone(),
            queue_type,
            queue_id,
            vertex_array_object
        };

        Ok(RafxQueueGl {
            inner: Arc::new(inner),
        })
    }

    pub fn wait_for_queue_idle(&self) -> RafxResult<()> {
        // There is no reason to wait for idle in GL
        Ok(())
    }

    fn submit_semaphore_wait(
        &self,
        wait_semaphores: &[&RafxSemaphoreGl],
    ) -> RafxResult<()> {
        if wait_semaphores.is_empty() {
            return Ok(());
        }

        let mut should_flush = false;
        for &semaphore in wait_semaphores {
            if semaphore.signal_available() {
                should_flush = true;
                semaphore.set_signal_available(false);
            }
        }

        if should_flush {
            self.device_context().gl_context().gl_flush()?;
        }

        Ok(())
    }

    pub fn submit(
        &self,
        command_buffers: &[&RafxCommandBufferGl],
        wait_semaphores: &[&RafxSemaphoreGl],
        signal_semaphores: &[&RafxSemaphoreGl],
        signal_fence: Option<&RafxFenceGl>,
    ) -> RafxResult<()> {
        assert!(!command_buffers.is_empty());

        self.submit_semaphore_wait(wait_semaphores)?;

        for semaphore in signal_semaphores {
            semaphore.set_signal_available(true);
        }

        if let Some(fence) = signal_fence {
            fence.set_submitted(true);
        }

        Ok(())
    }

    pub fn present(
        &self,
        swapchain: &RafxSwapchainGl,
        wait_semaphores: &[&RafxSemaphoreGl],
        _image_index: u32,
    ) -> RafxResult<RafxPresentSuccessResult> {
        self.submit_semaphore_wait(wait_semaphores)?;

        self.device_context().gl_context().gl_disable(crate::gl::gles20::SCISSOR_TEST)?;

        let surface_context = swapchain.surface_context();
        let gl_context_manager = self.device_context().gl_context_manager();
        gl_context_manager.set_current_context(Some(surface_context));
        self.device_context().gl_context().swap_buffers();
        gl_context_manager.set_current_context(Some(gl_context_manager.main_context()));

        Ok(RafxPresentSuccessResult::Success)
    }
}
