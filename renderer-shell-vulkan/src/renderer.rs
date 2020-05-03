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
use crate::{VkContext, VkDeviceContext};
//use crate::submit::PendingCommandBuffer;

/// May be implemented to get callbacks related to the renderer and framebuffer usage
pub trait RendererEventListener {
    /// Called whenever the swapchain needs to be created (the first time, and in cases where the
    /// swapchain needs to be recreated)
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()>;

    /// Called whenever the swapchain will be destroyed (when renderer is dropped, and also in cases
    /// where the swapchain needs to be recreated)
    fn swapchain_destroyed(&mut self);

    /// Called when we are presenting a new frame. The returned command buffer will be submitted
    /// with command buffers for the skia canvas
    fn render(
        &mut self,
        window: &dyn Window,
        device_context: &VkDeviceContext,
        present_index: usize,
    ) -> VkResult<Vec<vk::CommandBuffer>>;
}

/// Sets up a vulkan instance, device, and swapchain. Sends callbacks to a RendererEventListener
/// provided by the end user
pub struct Renderer {
    device_context: VkDeviceContext,
    physical_device: vk::PhysicalDevice,
    swapchain: ManuallyDrop<VkSwapchain>,
    present_mode_priority: Vec<PresentMode>,

    // Increase until > MAX_FRAMES_IN_FLIGHT, then set to 0, or -1 if no frame drawn yet
    sync_frame_index: usize,

    previous_inner_size: PhysicalSize,

    // This is set to false until tear_down is called. We don't use "normal" drop because the user
    // may need to pass in an EventListener to get cleanup callbacks. We still hook drop and if
    // torn_down is false, we can log an error.
    torn_down: bool
}

impl Renderer {
    /// Create the renderer
    pub fn new(
        context: &VkContext,
        window: &dyn Window,
        event_listener: Option<&mut dyn RendererEventListener>
    ) -> VkResult<Renderer> {
        let swapchain = ManuallyDrop::new(VkSwapchain::new(
            &context.device().device_context,
            window,
            None,
            context.present_mode_priority(),
        )?);

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_created(&context.device().device_context, &swapchain)?;
        }

        let sync_frame_index = 0;

        let previous_inner_size = window.physical_size();

        Ok(Renderer {
            device_context: context.device().device_context.clone(),
            physical_device: context.device().physical_device,
            swapchain,
            sync_frame_index,
            previous_inner_size,
            present_mode_priority: context.present_mode_priority().clone(),
            //event_listeners,
            torn_down: false
        })
    }

    pub fn tear_down(&mut self, event_listener: Option<&mut dyn RendererEventListener>) {
        unsafe {
            self.device_context.device().device_wait_idle().unwrap();
        }

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_destroyed();
        }

        // self will drop
        self.torn_down = true;
    }



    // pub fn device(&self) -> &VkDevice {
    //     &self.device
    // }

    // pub fn device_mut(&mut self) -> &mut VkDevice {
    //     &mut self.device
    // }

    /// Call to render a frame. This can block for certain presentation modes. This will rebuild
    /// the swapchain if necessary.
    pub fn draw(
        &mut self,
        window: &dyn Window,
        mut event_listener: Option<&mut dyn RendererEventListener>
    ) -> VkResult<()> {
        if window.physical_size() != self.previous_inner_size {
            debug!("Detected window inner size change, rebuilding swapchain");
            self.rebuild_swapchain(window, &mut event_listener)?;
        }

        let result = self.do_draw(window, &mut event_listener);
        if let Err(e) = result {
            match e {
                ash::vk::Result::ERROR_OUT_OF_DATE_KHR => self.rebuild_swapchain(window, &mut event_listener),
                ash::vk::Result::SUCCESS => Ok(()),
                ash::vk::Result::SUBOPTIMAL_KHR => Ok(()),
                _ => {
                    warn!("Unexpected rendering error");
                    Err(e)
                }
            }
        } else {
            Ok(())
        }
    }

    fn rebuild_swapchain(
        &mut self,
        window: &dyn Window,
        event_listener: &mut Option<&mut dyn RendererEventListener>
    ) -> VkResult<()> {
        // Let event listeners know the swapchain will be destroyed
        unsafe {
            self.device_context.device().device_wait_idle()?;
            if let Some(event_listener) = event_listener {
                event_listener.swapchain_destroyed();
            }
        }

        let new_swapchain = ManuallyDrop::new(VkSwapchain::new(
            &self.device_context,
            window,
            Some(self.swapchain.swapchain),
            &self.present_mode_priority,
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

    /// Do the render
    fn do_draw(
        &mut self,
        window: &dyn Window,
        event_listener: &mut Option<&mut dyn RendererEventListener>
    ) -> VkResult<()> {
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

        let (present_index, _is_suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.swapchain.image_available_semaphores[self.sync_frame_index],
                vk::Fence::null(),
            )?
        };

        let mut command_buffers = vec![];
        if let Some(event_listener) = event_listener {
            let mut buffers = event_listener.render(window, &self.device_context, present_index as usize)?;
            command_buffers.append(&mut buffers);
        }

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
        let image_indices = [present_index];
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
}

impl Drop for Renderer {
    fn drop(&mut self) {
        debug!("destroying Renderer");

        // This checks that the device is idle and issues swapchain_destroyed to the event listener
        assert!(self.torn_down);

        unsafe {
            ManuallyDrop::drop(&mut self.swapchain);
        }

        debug!("destroyed Renderer");
    }
}
