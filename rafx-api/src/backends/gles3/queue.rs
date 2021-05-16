use crate::gles3::{
    gles3_bindings, RafxCommandBufferGles3, RafxCommandPoolGles3, RafxDeviceContextGles3,
    RafxFenceGles3, RafxSemaphoreGles3, RafxSwapchainGles3, NONE_FRAMEBUFFER,
};
use crate::{RafxCommandPoolDef, RafxPresentSuccessResult, RafxQueueType, RafxResult};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static NEXT_QUEUE_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
pub struct RafxQueueGles3Inner {
    device_context: RafxDeviceContextGles3,
    queue_type: RafxQueueType,
    queue_id: u32,
}

#[derive(Clone, Debug)]
pub struct RafxQueueGles3 {
    inner: Arc<RafxQueueGles3Inner>,
}

impl RafxQueueGles3 {
    pub fn queue_id(&self) -> u32 {
        self.inner.queue_id
    }

    pub fn queue_type(&self) -> RafxQueueType {
        self.inner.queue_type
    }

    pub fn device_context(&self) -> &RafxDeviceContextGles3 {
        &self.inner.device_context
    }

    pub fn create_command_pool(
        &self,
        command_pool_def: &RafxCommandPoolDef,
    ) -> RafxResult<RafxCommandPoolGles3> {
        RafxCommandPoolGles3::new(&self, command_pool_def)
    }

    pub fn new(
        device_context: &RafxDeviceContextGles3,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueGles3> {
        let queue_id = NEXT_QUEUE_ID.fetch_add(1, Ordering::Relaxed);
        let inner = RafxQueueGles3Inner {
            device_context: device_context.clone(),
            queue_type,
            queue_id,
        };

        Ok(RafxQueueGles3 {
            inner: Arc::new(inner),
        })
    }

    pub fn wait_for_queue_idle(&self) -> RafxResult<()> {
        // There is no reason to wait for idle in GL
        Ok(())
    }

    fn submit_semaphore_wait(
        &self,
        wait_semaphores: &[&RafxSemaphoreGles3],
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
        command_buffers: &[&RafxCommandBufferGles3],
        wait_semaphores: &[&RafxSemaphoreGles3],
        signal_semaphores: &[&RafxSemaphoreGles3],
        signal_fence: Option<&RafxFenceGles3>,
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
        swapchain: &RafxSwapchainGles3,
        wait_semaphores: &[&RafxSemaphoreGles3],
        _image_index: u32,
    ) -> RafxResult<RafxPresentSuccessResult> {
        self.submit_semaphore_wait(wait_semaphores)?;

        let gl_context = self.device_context().gl_context();
        gl_context.gl_bind_framebuffer(gles3_bindings::FRAMEBUFFER, NONE_FRAMEBUFFER)?;

        self.device_context().inner.fullscreen_quad.draw(
            gl_context,
            &swapchain.swapchain_image,
            true,
        )?;

        let surface_context = swapchain.surface_context();
        let gl_context_manager = self.device_context().gl_context_manager();
        gl_context_manager.set_current_context(Some(surface_context));
        self.device_context().gl_context().swap_buffers();
        gl_context_manager.set_current_context(Some(gl_context_manager.main_context()));

        Ok(RafxPresentSuccessResult::Success)
    }
}
