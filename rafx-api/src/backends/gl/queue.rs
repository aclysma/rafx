use crate::gl::{
    RafxDeviceContextGl, RafxFenceGl, RafxSemaphoreGl, RafxSwapchainGl,
};
use crate::{RafxCommandPoolDef, RafxPresentSuccessResult, RafxQueueType, RafxResult};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

static NEXT_QUEUE_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
pub struct RafxQueueGlInner {
    device_context: RafxDeviceContextGl,
    queue_type: RafxQueueType,
    //queue: gl_rs::CommandQueue,
    queue_id: u32,
    //barrier_flags: AtomicU8,
    //fence: gl_rs::Fence,
}

// for gl_rs::CommandQueue
unsafe impl Send for RafxQueueGlInner {}
unsafe impl Sync for RafxQueueGlInner {}

#[derive(Clone, Debug)]
pub struct RafxQueueGl {
    inner: Arc<RafxQueueGlInner>,
}

impl RafxQueueGl {
    pub fn queue_id(&self) -> u32 {
        self.inner.queue_id
    }

    // //
    // // These barrier flag helpers are not meant to be threadsafe, the Rafx API assumes command
    // // buffers and the queues they come from are not concurrently accessed. (Even if we were careful
    // // about thread-safety here, underlying GPU APIs will often produce undefined behavior if this
    // // occurs.)
    // //
    // pub fn barrier_flags(&self) -> BarrierFlagsGl {
    //     BarrierFlagsGl::from_bits(self.inner.barrier_flags.load(Ordering::Relaxed)).unwrap()
    // }
    //
    // // Get the fence used internally by the currently recording command buffer
    // pub fn gl_fence(&self) -> &gl_rs::FenceRef {
    //     self.inner.fence.as_ref()
    // }

    // pub fn add_barrier_flags(
    //     &self,
    //     flags: BarrierFlagsGl,
    // ) {
    //     self.inner
    //         .barrier_flags
    //         .fetch_or(flags.bits(), Ordering::Relaxed);
    // }
    //
    // pub fn clear_barrier_flags(&self) {
    //     self.inner.barrier_flags.store(0, Ordering::Relaxed);
    // }

    pub fn queue_type(&self) -> RafxQueueType {
        self.inner.queue_type
    }

    pub fn device_context(&self) -> &RafxDeviceContextGl {
        &self.inner.device_context
    }

    // pub fn gl_queue(&self) -> &gl_rs::CommandQueueRef {
    //     self.inner.queue.as_ref()
    // }

    // pub fn create_command_pool(
    //     &self,
    //     command_pool_def: &RafxCommandPoolDef,
    // ) -> RafxResult<RafxCommandPoolGl> {
    //     RafxCommandPoolGl::new(&self, command_pool_def)
    // }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueGl> {
        //let queue = device_context.device().new_command_queue();
        //let fence = device_context.device().new_fence();

        let queue_id = NEXT_QUEUE_ID.fetch_add(1, Ordering::Relaxed);
        let inner = RafxQueueGlInner {
            device_context: device_context.clone(),
            queue_type,
            //queue,
            queue_id,
            //barrier_flags: AtomicU8::default(),
            //fence,
        };

        Ok(RafxQueueGl {
            inner: Arc::new(inner),
        })
    }

    pub fn wait_for_queue_idle(&self) -> RafxResult<()> {
        let wait = self
            .inner
            .queue
            .new_command_buffer_with_unretained_references();
        unimplemented!();
        // wait.commit();
        // wait.wait_until_completed();
        Ok(())
    }

    fn submit_semaphore_wait(
        &self,
        wait_semaphores: &[&RafxSemaphoreGl],
    ) {
        unimplemented!();
        // let wait_command_buffer_required = wait_semaphores.iter().any(|x| x.signal_available());
        //
        // if wait_command_buffer_required {
        //     let wait_command_buffer = self
        //         .inner
        //         .queue
        //         .new_command_buffer_with_unretained_references();
        //     for wait_semaphore in wait_semaphores {
        //         if wait_semaphore.signal_available() {
        //             wait_command_buffer.encode_wait_for_event(wait_semaphore.gl_event(), 1);
        //             wait_semaphore.set_signal_available(false);
        //         }
        //     }
        //
        //     wait_command_buffer.commit();
        // }
    }

    pub fn submit(
        &self,
        command_buffers: &[&RafxCommandBufferGl],
        wait_semaphores: &[&RafxSemaphoreGl],
        signal_semaphores: &[&RafxSemaphoreGl],
        signal_fence: Option<&RafxFenceGl>,
    ) -> RafxResult<()> {
        // objc::rc::autoreleasepool(|| {
        //     assert!(!command_buffers.is_empty());
        //
        //     // If a signal fence exists, mark it as submitted and add a closure to execute at the end
        //     // of each command buffer. The closure will have a shared atomic counter that is incremented
        //     // each time it is executed. When the counter reaches the number of command buffers, we know
        //     // that all command buffers are finished executing and will signal the semaphore
        //     if let Some(signal_fence) = signal_fence {
        //         signal_fence.set_submitted(true);
        //
        //         let command_count = command_buffers.len();
        //         let complete_count = Arc::new(AtomicUsize::new(0));
        //         let dispatch_semaphore = signal_fence.gl_dispatch_semaphore().clone();
        //         let block = block::ConcreteBlock::new(move |_command_buffer_ref| {
        //             // Add 1 because fetch_add returns the value from before the add
        //             let complete =
        //                 complete_count.fetch_add(1, Ordering::Relaxed) + 1 == command_count;
        //             if complete {
        //                 dispatch_semaphore.signal();
        //             }
        //         })
        //         .copy();
        //
        //         for command_buffer in command_buffers {
        //             command_buffer
        //                 .gl_command_buffer()
        //                 .unwrap()
        //                 .add_completed_handler(&block);
        //         }
        //     }
        //
        //     for signal_semaphore in signal_semaphores {
        //         command_buffers
        //             .last()
        //             .unwrap()
        //             .gl_command_buffer()
        //             .unwrap()
        //             .encode_signal_event(signal_semaphore.gl_event(), 1);
        //         signal_semaphore.set_signal_available(true);
        //     }
        //
        //     self.submit_semaphore_wait(wait_semaphores);
        //
        //     for command_buffer in command_buffers {
        //         command_buffer.end_current_encoders(false)?;
        //         command_buffer.gl_command_buffer().unwrap().commit();
        //         command_buffer.clear_command_buffer();
        //     }
        //
        //     Ok(())
        // })
        unimplemented!()
    }

    pub fn present(
        &self,
        swapchain: &RafxSwapchainGl,
        wait_semaphores: &[&RafxSemaphoreGl],
        _image_index: u32,
    ) -> RafxResult<RafxPresentSuccessResult> {
        unimplemented!()
        // objc::rc::autoreleasepool(|| {
        //     self.submit_semaphore_wait(wait_semaphores);
        //
        //     let command_buffer = self.inner.queue.new_command_buffer();
        //     let drawable = swapchain.take_drawable().unwrap();
        //     command_buffer.present_drawable(drawable.as_ref());
        //     // Invalidate swapchain texture in some way?
        //     command_buffer.commit();
        //
        //     Ok(RafxPresentSuccessResult::Success)
        // })
    }
}
