#[macro_use]
extern crate log;

pub use ash;

use ash::vk;

mod alignment;
use alignment::Align;

pub mod util;

mod window_support;
pub use window_support::Window;

mod instance;
pub use instance::VkInstance;
pub use instance::VkCreateInstanceError;

mod device;
pub use device::VkDevice;
pub use device::VkQueueFamilyIndices;
pub use device::VkQueues;

mod swapchain;
pub use swapchain::VkSwapchain;
pub use swapchain::SwapchainInfo;
pub use swapchain::MAX_FRAMES_IN_FLIGHT;

mod buffer;
pub use buffer::VkBuffer;

mod debug_reporter;
pub use debug_reporter::VkDebugReporter;

#[allow(clippy::module_inception)]
mod renderer;
pub use renderer::RendererBuilder;
pub use renderer::Renderer;
pub use renderer::CreateRendererError;
pub use renderer::RendererPlugin;

mod coordinates;
pub use coordinates::Size;
pub use coordinates::LogicalSize;
pub use coordinates::PhysicalSize;

pub mod slab;

/// Used to select which PresentMode is preferred. Some of this is hardware/platform dependent and
/// it's a good idea to read the Vulkan spec.
///
/// `Fifo` is always available on Vulkan devices that comply with the spec and is a good default for
/// many cases.
///
/// Values here match VkPresentModeKHR
#[derive(Copy, Clone, Debug)]
pub enum PresentMode {
    /// (`VK_PRESENT_MODE_IMMEDIATE_KHR`) - No internal buffering, and can result in screen
    /// tearin.
    Immediate = 0,

    /// (`VK_PRESENT_MODE_MAILBOX_KHR`) - This allows rendering as fast as the hardware will
    /// allow, but queues the rendered images in a way that avoids tearing. In other words, if the
    /// hardware renders 10 frames within a single vertical blanking period, the first 9 will be
    /// dropped. This is the best choice for lowest latency where power consumption is not a
    /// concern.
    Mailbox = 1,

    /// (`VK_PRESENT_MODE_FIFO_KHR`) - Default option, guaranteed to be available, and locks
    /// screen draw to vsync. This is a good default choice generally, and more power efficient
    /// than mailbox, but can have higher latency than mailbox.
    Fifo = 2,

    /// (`VK_PRESENT_MODE_FIFO_RELAXED_KHR`) - Similar to Fifo but if rendering is late,
    /// screen tearing can be observed.
    FifoRelaxed = 3,
}

impl PresentMode {
    /// Convert to `vk::PresentModeKHR`
    pub fn to_vk(self) -> vk::PresentModeKHR {
        match self {
            PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
            PresentMode::Fifo => vk::PresentModeKHR::FIFO,
            PresentMode::FifoRelaxed => vk::PresentModeKHR::FIFO_RELAXED,
        }
    }
}

/// Used to specify which type of physical device is preferred. It's recommended to read the Vulkan
/// spec to understand precisely what these types mean
///
/// Values here match VkPhysicalDeviceType, DiscreteGpu is the recommended default
#[derive(Copy, Clone, Debug)]
pub enum PhysicalDeviceType {
    /// Corresponds to `VK_PHYSICAL_DEVICE_TYPE_OTHER`
    Other = 0,

    /// Corresponds to `VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU`
    IntegratedGpu = 1,

    /// Corresponds to `VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU`
    DiscreteGpu = 2,

    /// Corresponds to `VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU`
    VirtualGpu = 3,

    /// Corresponds to `VK_PHYSICAL_DEVICE_TYPE_CPU`
    Cpu = 4,
}

impl PhysicalDeviceType {
    /// Convert to `vk::PhysicalDeviceType`
    pub fn to_vk(self) -> vk::PhysicalDeviceType {
        match self {
            PhysicalDeviceType::Other => vk::PhysicalDeviceType::OTHER,
            PhysicalDeviceType::IntegratedGpu => vk::PhysicalDeviceType::INTEGRATED_GPU,
            PhysicalDeviceType::DiscreteGpu => vk::PhysicalDeviceType::DISCRETE_GPU,
            PhysicalDeviceType::VirtualGpu => vk::PhysicalDeviceType::VIRTUAL_GPU,
            PhysicalDeviceType::Cpu => vk::PhysicalDeviceType::CPU,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ScaleToFit {
    Fill = 0,
    Start = 1,
    Center = 2,
    End = 3,
}

#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Default coordinate system to use
#[derive(Copy, Clone)]
pub enum CoordinateSystem {
    /// Logical coordinates will use (0,0) top-left and (+X,+Y) right-bottom where X/Y is the logical
    /// size of the window. Logical size applies a multiplier for hi-dpi displays. For example, many
    /// 4K displays would probably have a high-dpi factor of 2.0, simulating a 1080p display.
    Logical,

    /// Physical coordinates will use (0,0) top-left and (+X,+Y) right-bottom where X/Y is the raw
    /// number of pixels.
    Physical,

    /// Visible range allows specifying an arbitrary coordinate system. For example, if you want X to
    /// range (100, 300) and Y to range (-100, 400), you can do that. It's likely you'd want to
    /// determine either X or Y using the aspect ratio to avoid stretching.
    VisibleRange(Rect, ScaleToFit),

    /// FixedWidth will use the given center position and width, and calculate appropriate Y extents
    /// for the current aspect ratio
    FixedWidth(Point, f32),

    /// Do not modify the canvas matrix
    None,
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        CoordinateSystem::Logical
    }
}

