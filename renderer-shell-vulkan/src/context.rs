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
use crate::VkDeviceContext;
//use crate::submit::PendingCommandBuffer;

/// A builder to create the renderer. It's easier to use AppBuilder and implement an AppHandler, but
/// initializing the renderer and maintaining the window yourself allows for more customization
#[derive(Default)]
pub struct VkContextBuilder {
    app_name: CString,
    validation_layer_debug_report_flags: vk::DebugReportFlagsEXT,
    present_mode_priority: Vec<PresentMode>,
    physical_device_type_priority: Vec<PhysicalDeviceType>,
}

impl VkContextBuilder {
    /// Construct the renderer builder with default options
    pub fn new() -> Self {
        VkContextBuilder {
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
    ) -> Result<VkContext, VkCreateContextError> {
        VkContext::new(
            &self.app_name,
            window,
            self.validation_layer_debug_report_flags,
            self.physical_device_type_priority.clone(),
            self.present_mode_priority.clone(),
        )
    }
}

/// Represents an error from creating the renderer
#[derive(Debug)]
pub enum VkCreateContextError {
    CreateInstanceError(VkCreateInstanceError),
    CreateDeviceError(VkCreateDeviceError),
    VkError(vk::Result),
}

impl std::error::Error for VkCreateContextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            VkCreateContextError::CreateInstanceError(ref e) => Some(e),
            VkCreateContextError::CreateDeviceError(ref e) => Some(e),
            VkCreateContextError::VkError(ref e) => Some(e),
        }
    }
}

impl core::fmt::Display for VkCreateContextError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            VkCreateContextError::CreateInstanceError(ref e) => e.fmt(fmt),
            VkCreateContextError::CreateDeviceError(ref e) => e.fmt(fmt),
            VkCreateContextError::VkError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<VkCreateInstanceError> for VkCreateContextError {
    fn from(result: VkCreateInstanceError) -> Self {
        VkCreateContextError::CreateInstanceError(result)
    }
}

impl From<VkCreateDeviceError> for VkCreateContextError {
    fn from(result: VkCreateDeviceError) -> Self {
        VkCreateContextError::CreateDeviceError(result)
    }
}

impl From<vk::Result> for VkCreateContextError {
    fn from(result: vk::Result) -> Self {
        VkCreateContextError::VkError(result)
    }
}

/// Sets up a vulkan instance, device, and swapchain. Sends callbacks to a RendererEventListener
/// provided by the end user. When the VkContext is dropped, all vulkan resources are torn down.
/// Most code will not need access to VkContext.. it's better to pass around VkDeviceContext. These
/// can be cloned and owned by value. However, all VkDeviceContexts must be dropped before dropping
/// the VkContext
pub struct VkContext {
    instance: ManuallyDrop<VkInstance>,
    device: ManuallyDrop<VkDevice>,

    present_mode_priority: Vec<PresentMode>,
}

impl VkContext {
    /// Create the renderer
    pub fn new(
        app_name: &CString,
        window: &dyn Window,
        validation_layer_debug_report_flags: vk::DebugReportFlagsEXT,
        physical_device_type_priority: Vec<PhysicalDeviceType>,
        present_mode_priority: Vec<PresentMode>,
    ) -> Result<VkContext, VkCreateContextError> {
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

        Ok(VkContext {
            instance,
            device,
            present_mode_priority
        })
    }

    pub fn instance(&self) -> &VkInstance { &self.instance }

    pub fn device(&self) -> &VkDevice {
        &self.device
    }

    pub fn device_context(&self) -> &VkDeviceContext {
        &self.device.device_context
    }

    pub fn present_mode_priority(&self) -> &Vec<PresentMode> { &self.present_mode_priority }
}

impl Drop for VkContext {
    fn drop(&mut self) {
        debug!("destroying Context");

        unsafe {
            ManuallyDrop::drop(&mut self.device);
            ManuallyDrop::drop(&mut self.instance);
        }

        debug!("destroyed Context");
    }
}
