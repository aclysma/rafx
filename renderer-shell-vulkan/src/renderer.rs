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
//use crate::submit::PendingCommandBuffer;

/// May be implemented to get callbacks related to the renderer and framebuffer usage
pub trait RendererEventListener {
    /// Called whenever the swapchain needs to be created (the first time, and in cases where the
    /// swapchain needs to be recreated)
    fn swapchain_created(
        &mut self,
        device: &VkDevice,
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
        device: &VkDevice,
        present_index: usize,
    ) -> VkResult<Vec<vk::CommandBuffer>>;
}

/// A builder to create the renderer. It's easier to use AppBuilder and implement an AppHandler, but
/// initializing the renderer and maintaining the window yourself allows for more customization
#[derive(Default)]
pub struct RendererBuilder {
    app_name: CString,
    validation_layer_debug_report_flags: vk::DebugReportFlagsEXT,
    present_mode_priority: Vec<PresentMode>,
    physical_device_type_priority: Vec<PhysicalDeviceType>,
}

impl RendererBuilder {
    /// Construct the renderer builder with default options
    pub fn new() -> Self {
        RendererBuilder {
            app_name: CString::new("RendererPrototype").unwrap(),
            validation_layer_debug_report_flags: vk::DebugReportFlagsEXT::all(),
            present_mode_priority: vec![PresentMode::Fifo],
            physical_device_type_priority: vec![
                PhysicalDeviceType::DiscreteGpu,
                PhysicalDeviceType::IntegratedGpu,
            ],
        }
    }

    /// Name of the app. This is passed into the vulkan layer. I believe it can hint things to the
    /// vulkan driver, but it's unlikely this makes a real difference. Still a good idea to set this
    /// to something meaningful though.
    pub fn app_name(
        mut self,
        app_name: CString,
    ) -> Self {
        self.app_name = app_name;
        self
    }

    /// If true, initialize the vulkan debug layers. This will require the vulkan SDK to be
    /// installed and the app will fail to launch if it isn't. This turns on ALL logging. For
    /// more control, see `validation_layer_debug_report_flags()`
    pub fn use_vulkan_debug_layer(
        self,
        use_vulkan_debug_layer: bool,
    ) -> Self {
        self.validation_layer_debug_report_flags(if use_vulkan_debug_layer {
            vk::DebugReportFlagsEXT::all()
        } else {
            vk::DebugReportFlagsEXT::empty()
        })
    }

    /// Sets the desired debug layer flags. If any flag is set, the vulkan debug layers will be
    /// loaded, which requires the Vulkan SDK to be installed. The app will fail to launch if it
    /// isn't.
    pub fn validation_layer_debug_report_flags(
        mut self,
        validation_layer_debug_report_flags: vk::DebugReportFlagsEXT,
    ) -> Self {
        self.validation_layer_debug_report_flags = validation_layer_debug_report_flags;
        self
    }

    /// Specify which PresentMode is preferred. Some of this is hardware/platform dependent and
    /// it's a good idea to read the Vulkan spec. You
    ///
    /// `present_mode_priority` should be a list of desired present modes, in descending order of
    /// preference. In other words, passing `[Mailbox, Fifo]` will direct Skulpin to use mailbox
    /// where available, but otherwise use `Fifo`.
    ///
    /// Since `Fifo` is always available, this is the mode that will be chosen if no desired mode is
    /// available.
    pub fn present_mode_priority(
        mut self,
        present_mode_priority: Vec<PresentMode>,
    ) -> Self {
        self.present_mode_priority = present_mode_priority;
        self
    }

    /// Specify which type of physical device is preferred. It's recommended to read the Vulkan spec
    /// to understand precisely what these types mean
    ///
    /// `physical_device_type_priority` should be a list of desired present modes, in descending
    /// order of preference. In other words, passing `[Discrete, Integrated]` will direct Skulpin to
    /// use the discrete GPU where available, otherwise integrated.
    ///
    /// If the desired device type can't be found, Skulpin will try to use whatever device is
    /// available. By default `Discrete` is favored, then `Integrated`, then anything that's
    /// available. It could make sense to favor `Integrated` over `Discrete` when minimizing
    /// power consumption is important. (Although I haven't tested this myself)
    pub fn physical_device_type_priority(
        mut self,
        physical_device_type_priority: Vec<PhysicalDeviceType>,
    ) -> Self {
        self.physical_device_type_priority = physical_device_type_priority;
        self
    }

    /// Easy shortcut to set device type priority to `Integrated`, then `Discrete`, then any.
    pub fn prefer_integrated_gpu(self) -> Self {
        self.physical_device_type_priority(vec![
            PhysicalDeviceType::IntegratedGpu,
            PhysicalDeviceType::DiscreteGpu,
        ])
    }

    /// Easy shortcut to set device type priority to `Discrete`, then `Integrated`, than any.
    /// (This is the default behavior)
    pub fn prefer_discrete_gpu(self) -> Self {
        self.physical_device_type_priority(vec![
            PhysicalDeviceType::DiscreteGpu,
            PhysicalDeviceType::IntegratedGpu,
        ])
    }

    /// Prefer using `Fifo` presentation mode. This presentation mode is always available on a
    /// device that complies with the vulkan spec.
    pub fn prefer_fifo_present_mode(self) -> Self {
        self.present_mode_priority(vec![PresentMode::Fifo])
    }

    /// Prefer using `Mailbox` presentation mode, and fall back to `Fifo` when not available.
    pub fn prefer_mailbox_present_mode(self) -> Self {
        self.present_mode_priority(vec![PresentMode::Mailbox, PresentMode::Fifo])
    }

    /// Builds the renderer. The window that's passed in will be used for creating the swapchain
    pub fn build(
        self,
        window: &dyn Window,
        event_listener: Option<&mut dyn RendererEventListener>
    ) -> Result<Renderer, CreateRendererError> {
        Renderer::new(
            &self.app_name,
            window,
            self.validation_layer_debug_report_flags,
            self.physical_device_type_priority.clone(),
            self.present_mode_priority.clone(),
            event_listener
        )
    }
}

/// Represents an error from creating the renderer
#[derive(Debug)]
pub enum CreateRendererError {
    CreateInstanceError(VkCreateInstanceError),
    CreateDeviceError(VkCreateDeviceError),
    VkError(vk::Result),
}

impl std::error::Error for CreateRendererError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            CreateRendererError::CreateInstanceError(ref e) => Some(e),
            CreateRendererError::CreateDeviceError(ref e) => Some(e),
            CreateRendererError::VkError(ref e) => Some(e),
        }
    }
}

impl core::fmt::Display for CreateRendererError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            CreateRendererError::CreateInstanceError(ref e) => e.fmt(fmt),
            CreateRendererError::CreateDeviceError(ref e) => e.fmt(fmt),
            CreateRendererError::VkError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<VkCreateInstanceError> for CreateRendererError {
    fn from(result: VkCreateInstanceError) -> Self {
        CreateRendererError::CreateInstanceError(result)
    }
}

impl From<VkCreateDeviceError> for CreateRendererError {
    fn from(result: VkCreateDeviceError) -> Self {
        CreateRendererError::CreateDeviceError(result)
    }
}

impl From<vk::Result> for CreateRendererError {
    fn from(result: vk::Result) -> Self {
        CreateRendererError::VkError(result)
    }
}

/// Sets up a vulkan instance, device, and swapchain. Sends callbacks to a RendererEventListener
/// provided by the end user
pub struct Renderer {
    instance: ManuallyDrop<VkInstance>,
    device: ManuallyDrop<VkDevice>,

    swapchain: ManuallyDrop<VkSwapchain>,

    // Increase until > MAX_FRAMES_IN_FLIGHT, then set to 0, or -1 if no frame drawn yet
    sync_frame_index: usize,

    present_mode_priority: Vec<PresentMode>,

    previous_inner_size: PhysicalSize,

    // This is set to false until tear_down is called. We don't use "normal" drop because the user
    // may need to pass in an EventListener to get cleanup callbacks. We still hook drop and if
    // torn_down is false, we can log an error.
    torn_down: bool
}

impl Renderer {
    /// Create the renderer
    pub fn new(
        app_name: &CString,
        window: &dyn Window,
        validation_layer_debug_report_flags: vk::DebugReportFlagsEXT,
        physical_device_type_priority: Vec<PhysicalDeviceType>,
        present_mode_priority: Vec<PresentMode>,
        event_listener: Option<&mut dyn RendererEventListener>
    ) -> Result<Renderer, CreateRendererError> {
        let instance = ManuallyDrop::new(VkInstance::new(
            window,
            app_name,
            validation_layer_debug_report_flags,
        )?);

        let device = ManuallyDrop::new(VkDevice::new(
            &instance,
            window,
            &physical_device_type_priority,
        )?);

        let swapchain = ManuallyDrop::new(VkSwapchain::new(
            &instance,
            &device,
            window,
            None,
            &present_mode_priority,
        )?);

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_created(&device, &swapchain)?;
        }

        let sync_frame_index = 0;

        let previous_inner_size = window.physical_size();

        Ok(Renderer {
            instance,
            device,
            swapchain,
            sync_frame_index,
            present_mode_priority,
            previous_inner_size,
            //event_listeners,
            torn_down: false
        })
    }

    pub fn tear_down(&mut self, event_listener: Option<&mut dyn RendererEventListener>) {
        unsafe {
            self.device.device().device_wait_idle().unwrap();
        }

        if let Some(event_listener) = event_listener {
            event_listener.swapchain_destroyed();
        }

        // self will drop
        self.torn_down = true;
    }



    pub fn device(&self) -> &VkDevice {
        &self.device
    }

    pub fn device_mut(&mut self) -> &mut VkDevice {
        &mut self.device
    }

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
            self.device.device().device_wait_idle()?;
            if let Some(event_listener) = event_listener {
                event_listener.swapchain_destroyed();
            }
        }

        let new_swapchain = ManuallyDrop::new(VkSwapchain::new(
            &self.instance,
            &self.device,
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
            event_listener.swapchain_created(&self.device, &self.swapchain)?;
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
            self.device
                .device()
                .wait_for_fences(&[frame_fence], true, std::u64::MAX)?;
            self.device.device().reset_fences(&[frame_fence])?;
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
            let mut buffers = event_listener.render(window, &self.device, present_index as usize)?;
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
            self.device.device().queue_submit(
                self.device.queues.graphics_queue,
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
                .queue_present(self.device.queues.present_queue, &present_info)?;
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
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.instance);
        }

        debug!("destroyed Renderer");
    }
}
