use ash::version::{DeviceV1_0};
use ash::prelude::VkResult;

use std::mem::ManuallyDrop;
use ash::vk;
use std::sync::atomic::{AtomicUsize, Ordering, AtomicBool};

use super::VkSwapchain;

use super::MAX_FRAMES_IN_FLIGHT;
use super::PresentMode;

use super::PhysicalSize;
use super::Window;
use crate::{VkContext, VkDeviceContext, MsaaLevel};
use std::sync::Arc;
use crossbeam_channel::{Sender, Receiver};
//use crate::submit::PendingCommandBuffer;

pub struct FrameInFlight {
    // These are used to detect frames being presented out of order
    sync_frame_index: usize,
    shared_sync_frame_index: Arc<AtomicUsize>,

    // Send the result from the previous frame back to the swapchain
    result_tx: Sender<VkResult<()>>,

    // This can be used to index into per-frame resources
    present_index: u32,

    // If this is true, consider re-creating the swapchain
    is_suboptimal: bool,

    // All the resources required to do the present
    device_context: VkDeviceContext,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    swapchain: vk::SwapchainKHR,
    swapchain_loader: ash::extensions::khr::Swapchain,
}

impl FrameInFlight {
    // A value that stays in step with the image index returned by the swapchain. There is no
    // guarantee on the ordering of present image index (i.e. it may decrease). It is only promised
    // to not be in use by a frame in flight
    pub fn present_index(&self) -> u32 {
        self.present_index
    }

    // If true, consider recreating the swapchain
    pub fn is_suboptimal(&self) -> bool {
        self.is_suboptimal
    }

    // Can be called by the end user to end the frame early and defer a result to the next acquire
    // image call
    pub fn cancel_present(
        self,
        result: VkResult<()>,
    ) {
        //TODO: AFAIK there is no way to simply trigger the semaphore and skip calling do_present
        // with no command buffers. The downside of doing this is that we end up with both the
        // end user's result and a result from do_present and have no sensible way of merging them
        let _ = self.do_present(&[]);
        self.result_tx.send(result).unwrap();
    }

    // submit the given command buffers and preset the swapchain image for this frame
    pub fn present(
        self,
        command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<()> {
        let result = self.do_present(command_buffers);
        self.result_tx.send(result).unwrap();
        result
    }

    pub fn do_present(
        &self,
        command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<()> {
        // A present can only occur using the result from the previous acquire_next_image call
        let sync_frame_index = self.shared_sync_frame_index.load(Ordering::Relaxed);
        assert!(self.sync_frame_index == sync_frame_index);
        let frame_fence = self.in_flight_fence;

        let wait_semaphores = [self.image_available_semaphore];
        let signal_semaphores = [self.render_finished_semaphore];

        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        //add fence to queue submit
        let submit_info = [vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .signal_semaphores(&signal_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&command_buffers)
            .build()];

        unsafe {
            let queue = self.device_context.queues().graphics_queue.lock().unwrap();
            self.device_context
                .device()
                .queue_submit(*queue, &submit_info, frame_fence)?;
        }

        let wait_semaphors = [self.render_finished_semaphore];
        let swapchains = [self.swapchain];
        let image_indices = [self.present_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphors) // &base.rendering_complete_semaphore)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            let queue = self.device_context.queues().present_queue.lock().unwrap();
            self.swapchain_loader.queue_present(*queue, &present_info)?;
        }

        self.shared_sync_frame_index.store(
            (sync_frame_index + 1) % MAX_FRAMES_IN_FLIGHT,
            Ordering::Relaxed,
        );

        Ok(())
    }
}

/// May be implemented to get callbacks related to the swapchain being created/destroyed
pub trait VkSurfaceSwapchainLifetimeListener {
    /// Called whenever the swapchain needs to be created (the first time, and in cases where the
    /// swapchain needs to be recreated)
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()>;

    /// Called whenever the swapchain will be destroyed (when VkSurface is dropped, and also in cases
    /// where the swapchain needs to be recreated)
    fn swapchain_destroyed(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    );
}

/// Sets up a vulkan swapchain. Sends callbacks to a VkSurfaceEventListener provided by the end user
/// This struct can remain in use throughout the lifetime of a window. The swapchain contained
/// within (both the vk::Swapchain and VkSwapchain) may be destroyed/recreated as needed. This
/// struct has the logic to kick off recreating it.
pub struct VkSurface {
    device_context: VkDeviceContext,
    swapchain: ManuallyDrop<VkSwapchain>,
    present_mode_priority: Vec<PresentMode>,
    msaa_level_priority: Vec<MsaaLevel>,

    // Increase until > MAX_FRAMES_IN_FLIGHT, then set to 0, or -1 if no frame drawn yet
    sync_frame_index: Arc<AtomicUsize>,

    previous_inner_size: PhysicalSize,

    frame_is_in_flight: AtomicBool,
    result_tx: Sender<VkResult<()>>,
    result_rx: Receiver<VkResult<()>>,

    // This is set to false until tear_down is called. We don't use "normal" drop because the user
    // may need to pass in an EventListener to get cleanup callbacks. We still hook drop and if
    // torn_down is false, we can log an error.
    torn_down: bool,
}

impl VkSurface {
    /// Create the surface - a per-window object that maintains the swapchain
    pub fn new(
        context: &VkContext,
        window: &dyn Window,
        event_listener: Option<&mut dyn VkSurfaceSwapchainLifetimeListener>,
    ) -> VkResult<VkSurface> {
        let swapchain = ManuallyDrop::new(VkSwapchain::new(
            &context.device().device_context,
            window,
            None,
            context.present_mode_priority(),
            context.msaa_level_priority(),
        )?);

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_created(&context.device().device_context, &swapchain)?;
        }

        let sync_frame_index = AtomicUsize::new(0);

        let previous_inner_size = window.physical_size();

        let (result_tx, result_rx) = crossbeam_channel::bounded(1);

        Ok(VkSurface {
            device_context: context.device().device_context.clone(),
            swapchain,
            sync_frame_index: Arc::new(sync_frame_index),
            previous_inner_size,
            present_mode_priority: context.present_mode_priority().clone(),
            msaa_level_priority: context.msaa_level_priority().clone(),
            frame_is_in_flight: AtomicBool::new(false),
            result_tx,
            result_rx,
            torn_down: false,
        })
    }

    pub fn tear_down(
        &mut self,
        event_listener: Option<&mut dyn VkSurfaceSwapchainLifetimeListener>,
    ) {
        self.wait_until_frame_not_in_flight().unwrap();
        unsafe {
            self.device_context.device().device_wait_idle().unwrap();
        }

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_destroyed(&self.device_context, &self.swapchain);
        }

        unsafe {
            ManuallyDrop::drop(&mut self.swapchain);
        }

        // self will drop
        self.torn_down = true;
    }

    // If a frame is in flight, block until it completes
    pub fn wait_until_frame_not_in_flight(&self) -> VkResult<()> {
        if self.frame_is_in_flight.load(Ordering::Relaxed) {
            self.frame_is_in_flight.store(false, Ordering::Relaxed);
            self.result_rx.recv().unwrap()?;
        }

        Ok(())
    }

    pub fn acquire_next_swapchain_image(
        &mut self,
        window: &dyn Window,
    ) -> VkResult<FrameInFlight> {
        // If a frame is still outstanding from a previous acquire_next_swapchain_image call, wait
        // to receive the result of that frame. If the result was an error, return that error now.
        // This allows us to handle errors from the render thread in the main thread. This wait is
        // only blocking on getting the previous frame submitted. It's possible the GPU is still
        // processing it, and even the frame before it.
        self.wait_until_frame_not_in_flight()?;

        if window.physical_size() != self.previous_inner_size {
            return Err(vk::Result::ERROR_OUT_OF_DATE_KHR);
        }

        let sync_frame_index = self.sync_frame_index.load(Ordering::Relaxed);
        let frame_fence = self.swapchain.in_flight_fences[sync_frame_index];

        // Wait if the GPU is already processing too many frames
        unsafe {
            //TODO: Dont lock up forever (don't use std::u64::MAX)
            self.device_context
                .device()
                .wait_for_fences(&[frame_fence], true, std::u64::MAX)?;
            self.device_context.device().reset_fences(&[frame_fence])?;
        }

        let (present_index, is_suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.swapchain.image_available_semaphores[sync_frame_index],
                vk::Fence::null(),
            )?
        };

        self.frame_is_in_flight.store(true, Ordering::Relaxed);

        Ok(FrameInFlight {
            sync_frame_index,
            shared_sync_frame_index: self.sync_frame_index.clone(),
            present_index,
            is_suboptimal,
            result_tx: self.result_tx.clone(),

            device_context: self.device_context.clone(),
            image_available_semaphore: self.swapchain.image_available_semaphores[sync_frame_index],
            render_finished_semaphore: self.swapchain.render_finished_semaphores[sync_frame_index],
            in_flight_fence: self.swapchain.in_flight_fences[sync_frame_index],
            swapchain: self.swapchain.swapchain,
            swapchain_loader: self.swapchain.swapchain_loader.clone(),
        })
    }

    pub fn rebuild_swapchain(
        &mut self,
        window: &dyn Window,
        event_listener: &mut Option<&mut dyn VkSurfaceSwapchainLifetimeListener>,
    ) -> VkResult<()> {
        // If a frame_in_flight from a previous acquire_next_swapchain_image() call is still
        // outstanding, wait for it to finish. It is referencing resources that we are about to
        // destroy.
        //
        // Ignore this error, generally rebuilding the swapchain means we are wanting to reset
        // all the state anyways. //TODO: Certain fatal may need to be returned, even here.
        let _ = self.wait_until_frame_not_in_flight();

        // Let event listeners know the swapchain will be destroyed
        unsafe {
            self.device_context.device().device_wait_idle()?;
            if let Some(event_listener) = event_listener {
                event_listener.swapchain_destroyed(&self.device_context, &self.swapchain);
            }
        }

        let new_swapchain = ManuallyDrop::new(VkSwapchain::new(
            &self.device_context,
            window,
            Some(self.swapchain.swapchain),
            &self.present_mode_priority,
            &self.msaa_level_priority,
        )?);

        unsafe {
            ManuallyDrop::drop(&mut self.swapchain);
        }

        // Let even listeners know a new swapchain has been created
        self.swapchain = new_swapchain;
        if let Some(event_listener) = event_listener {
            event_listener.swapchain_created(&self.device_context, &self.swapchain)?;
        }

        self.previous_inner_size = window.physical_size();

        Ok(())
    }
}

impl Drop for VkSurface {
    fn drop(&mut self) {
        trace!("destroying VkSurface");

        // This checks that the device is idle and issues swapchain_destroyed to the event listener
        assert!(self.torn_down);

        trace!("destroyed VkSurface");
    }
}
