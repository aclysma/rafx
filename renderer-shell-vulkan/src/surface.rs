use std::ffi::CString;

use ash::version::DeviceV1_0;
use ash::prelude::VkResult;

use std::mem::ManuallyDrop;
use ash::vk;

use super::VkInstance;
use super::VkCreateInstanceError;
use super::VkCreateDeviceError;
use super::VkDevice;
use super::VkSwapchain;

use super::MAX_FRAMES_IN_FLIGHT;
use super::PresentMode;
use super::PhysicalDeviceType;
use super::PhysicalSize;
use super::Window;
use crate::{VkContext, VkDeviceContext, MsaaLevel};
//use crate::submit::PendingCommandBuffer;

pub struct FrameInFlight {
    sync_frame_index: usize,
    present_index: u32,
    is_suboptimal: bool,
}

impl FrameInFlight {
    // I'm not aware of any reason to expose this.
    // // A value guaranteed to increase up to MAX_FRAMES_IN_FLIGHT, then return to zero
    // fn sync_frame_index(&self) -> usize {
    //     self.sync_frame_index
    // }

    // A value that stays in step with the image index returned by the swapchain. There is no
    // guarantee on the ordering of present image index (i.e. it may decrease). It is only promised
    // to not be in use by a frame in flight
    fn present_index(&self) -> u32 {
        self.present_index
    }

    // If true, consider recreating the swapchain
    fn is_suboptimal(&self) -> bool {
        self.is_suboptimal
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
    sync_frame_index: usize,

    previous_inner_size: PhysicalSize,

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
            context.msaa_level_priority()
        )?);

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_created(&context.device().device_context, &swapchain)?;
        }

        let sync_frame_index = 0;

        let previous_inner_size = window.physical_size();

        Ok(VkSurface {
            device_context: context.device().device_context.clone(),
            swapchain,
            sync_frame_index,
            previous_inner_size,
            present_mode_priority: context.present_mode_priority().clone(),
            msaa_level_priority: context.msaa_level_priority().clone(),
            //event_listeners,
            torn_down: false,
        })
    }

    pub fn tear_down(
        &mut self,
        event_listener: Option<&mut dyn VkSurfaceSwapchainLifetimeListener>,
    ) {
        unsafe {
            self.device_context.device().device_wait_idle().unwrap();
        }

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_destroyed(&self.device_context, &self.swapchain);
        }

        // self will drop
        self.torn_down = true;
    }

    pub fn draw_with<T, F>(
        &mut self,
        mut event_listener: &mut T,
        window: &dyn Window,
        f: F
    ) -> VkResult<()>
        where
            T : VkSurfaceSwapchainLifetimeListener,
            F : FnOnce(&mut T, &VkDeviceContext, usize) -> VkResult<Vec<vk::CommandBuffer>>
    {
        let result = self.try_draw_with(event_listener, window, f);
        if let Err(e) = result {
            match e {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    self.rebuild_swapchain(window, &mut Some(event_listener))
                }
                ash::vk::Result::SUCCESS => Ok(()),
                ash::vk::Result::SUBOPTIMAL_KHR => Ok(()),
                //ash::vk::Result::TIMEOUT => Ok(()),
                _ => {
                    warn!("Unexpected rendering error");
                    Err(e)
                }
            }
        } else {
            Ok(())
        }
    }

    /// Do the render
    fn try_draw_with<T, F>(
        &mut self,
        event_listener: &mut T,
        window: &dyn Window,
        mut f: F
    ) -> VkResult<()>
        where
            T : VkSurfaceSwapchainLifetimeListener,
            F : FnOnce(&mut T, &VkDeviceContext, usize) -> VkResult<Vec<vk::CommandBuffer>>
    {
        let frame_in_flight = self.acquire_next_swapchain_image(window)?;

        let mut command_buffers = f(event_listener, &self.device_context, frame_in_flight.present_index as usize)?;

        self.present(window, frame_in_flight, &command_buffers)?;
        Ok(())
    }

    fn acquire_next_swapchain_image(
        &mut self,
        window: &dyn Window,
    ) -> VkResult<FrameInFlight> {
        if window.physical_size() != self.previous_inner_size {
            return Err(vk::Result::ERROR_OUT_OF_DATE_KHR);
        }

        let frame_fence = self.swapchain.in_flight_fences[self.sync_frame_index];

        //TODO: Dont lock up forever (don't use std::u64::MAX)
        //TODO: Can part of this run in a separate thread from the window pump?
        //TODO: Should we be passing along the sync_index instead of the present_index to downstream
        // event listeners?

        // Wait if two frame are already in flight
        unsafe {
            self.device_context
                .device()
                .wait_for_fences(&[frame_fence], true, std::u64::MAX)?;
            self.device_context.device().reset_fences(&[frame_fence])?;
        }

        let (present_index, is_suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.swapchain.image_available_semaphores[self.sync_frame_index],
                vk::Fence::null(),
            )?
        };

        Ok(FrameInFlight {
            sync_frame_index: self.sync_frame_index,
            present_index,
            is_suboptimal
        })
    }

    fn present(
        &mut self,
        window: &dyn Window,
        frame_in_flight: FrameInFlight,
        command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<()> {
        // A present can only occur using the result from the previous acquire_next_image call
        assert!(self.sync_frame_index == frame_in_flight.sync_frame_index);
        let frame_fence = self.swapchain.in_flight_fences[self.sync_frame_index];

        let wait_semaphores = [self.swapchain.image_available_semaphores[self.sync_frame_index]];
        let signal_semaphores = [self.swapchain.render_finished_semaphores[self.sync_frame_index]];

        let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        //add fence to queue submit
        let submit_info = [vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .signal_semaphores(&signal_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .command_buffers(&command_buffers)
            .build()];

        unsafe {
            self.device_context.device().queue_submit(
                self.device_context.queues().graphics_queue,
                &submit_info,
                frame_fence,
            )?;
        }

        let wait_semaphors = [self.swapchain.render_finished_semaphores[self.sync_frame_index]];
        let swapchains = [self.swapchain.swapchain];
        let image_indices = [frame_in_flight.present_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphors) // &base.rendering_complete_semaphore)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain
                .swapchain_loader
                .queue_present(self.device_context.queues().present_queue, &present_info)?;
        }

        self.sync_frame_index = (self.sync_frame_index + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    pub fn rebuild_swapchain(
        &mut self,
        window: &dyn Window,
        event_listener: &mut Option<&mut VkSurfaceSwapchainLifetimeListener>,
    ) -> VkResult<()>
    {
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
            &self.msaa_level_priority
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

        unsafe {
            ManuallyDrop::drop(&mut self.swapchain);
        }

        trace!("destroyed VkSurface");
    }
}
