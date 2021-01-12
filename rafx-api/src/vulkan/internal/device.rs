use ash::vk;

use ash::version::DeviceV1_0;

use std::sync::Arc;

#[cfg(debug_assertions)]
#[cfg(feature = "track-device-contexts")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

/// Used to select which PresentMode is preferred. Some of this is hardware/platform dependent and
/// it's a good idea to read the Vulkan spec.
///
/// `Fifo` is always available on Vulkan devices that comply with the spec and is a good default for
/// many cases.
///
/// Values here match VkPresentModeKHR
#[derive(Copy, Clone, Debug)]
pub enum VkPresentMode {
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

impl VkPresentMode {
    /// Convert to `vk::PresentModeKHR`
    pub fn to_vk(self) -> vk::PresentModeKHR {
        match self {
            VkPresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            VkPresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
            VkPresentMode::Fifo => vk::PresentModeKHR::FIFO,
            VkPresentMode::FifoRelaxed => vk::PresentModeKHR::FIFO_RELAXED,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct VkQueueFamilyIndices {
    pub graphics_queue_family_index: u32,
    pub compute_queue_family_index: u32,
    pub transfer_queue_family_index: u32,
}

#[derive(Clone)]
pub struct PhysicalDeviceInfo {
    pub score: i32,
    pub queue_family_indices: VkQueueFamilyIndices,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub extension_properties: Vec<ash::vk::ExtensionProperties>,
    pub all_queue_families: Vec<ash::vk::QueueFamilyProperties>,
}

pub struct VkDeviceContextInner {
    //entry: Arc<VkEntry>,
    //instance: ash::Instance,
    device: ash::Device,
    allocator: vk_mem::Allocator,
    //queue_allocator: VkQueueAllocatorSet,
    physical_device: vk::PhysicalDevice,
    physical_device_info: PhysicalDeviceInfo,
    default_present_mode_priority: Vec<VkPresentMode>,
    destroyed: AtomicBool,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    next_create_index: AtomicU64,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    all_contexts: Mutex<fnv::FnvHashMap<u64, backtrace::Backtrace>>,
}

impl VkDeviceContextInner {
    // Gets called by VkDevice when it is destroyed. This will be called one time for all cloned device contexts.
    unsafe fn try_destroy(self: Arc<Self>) -> Result<(), Arc<Self>> {
        match Arc::try_unwrap(self) {
            Ok(inner) => {
                std::mem::drop(inner);
                Ok(())
            }
            Err(arc) => Err(arc),
        }
    }
}

impl std::fmt::Debug for VkDeviceContextInner {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("VkDeviceContextInner")
            .field("device", &self.device.handle())
            .finish()
    }
}

impl Drop for VkDeviceContextInner {
    fn drop(&mut self) {
        if !self.destroyed.swap(true, Ordering::AcqRel) {
            unsafe {
                log::trace!("destroying device");
                self.allocator.destroy();
                self.device.destroy_device(None);
                //self.surface_loader.destroy_surface(self.surface, None);
                log::trace!("destroyed device");
            }
        }
    }
}

/// A lighter-weight structure that can be cached on downstream users. It includes
/// access to vk::Device and allocators.
//TODO: Rename to VkDevice
#[derive(Debug)]
pub struct VkDeviceContext {
    inner: Option<Arc<VkDeviceContextInner>>,
    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    create_index: u64,
}

impl Clone for VkDeviceContext {
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let create_index = {
            let create_index = self
                .inner
                .as_ref()
                .unwrap()
                .next_create_index
                .fetch_add(1, Ordering::Relaxed);
            // let create_backtrace = backtrace::Backtrace::new_unresolved();
            // self.inner
            //     .as_ref()
            //     .unwrap()
            //     .all_contexts
            //     .lock()
            //     .unwrap()
            //     .insert(create_index, create_backtrace);
            log::trace!("Cloned VkDeviceContext create_index {}", create_index);
            create_index
        };
        VkDeviceContext {
            inner: self.inner.clone(),
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index,
        }
    }
}

impl VkDeviceContext {
    pub fn device(&self) -> &ash::Device {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .device
    }

    pub fn allocator(&self) -> &vk_mem::Allocator {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .allocator
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .physical_device
    }

    pub fn physical_device_info(&self) -> &PhysicalDeviceInfo {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .physical_device_info
    }

    pub fn limits(&self) -> &vk::PhysicalDeviceLimits {
        &self.physical_device_info().properties.limits
    }

    pub fn queue_family_indices(&self) -> &VkQueueFamilyIndices {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .physical_device_info
            .queue_family_indices
    }

    pub fn default_present_mode_priority(&self) -> &[VkPresentMode] {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .default_present_mode_priority
    }

    pub fn new(
        default_present_mode_priority: Vec<VkPresentMode>,
        logical_device: ash::Device,
        allocator: vk_mem::Allocator,
        physical_device: vk::PhysicalDevice,
        physical_device_info: PhysicalDeviceInfo,
    ) -> Result<Self, VkCreateDeviceError> {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let all_contexts = {
            let create_backtrace = backtrace::Backtrace::new_unresolved();
            let mut all_contexts = fnv::FnvHashMap::<u64, backtrace::Backtrace>::default();
            all_contexts.insert(0, create_backtrace);
            all_contexts
        };

        let inner = VkDeviceContextInner {
            device: logical_device,
            allocator,
            physical_device,
            physical_device_info,
            default_present_mode_priority,
            destroyed: AtomicBool::new(false),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            all_contexts: Mutex::new(all_contexts),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            next_create_index: AtomicU64::new(1),
        };

        Ok(VkDeviceContext {
            inner: Some(Arc::new(inner)),
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index: 0,
        })
    }

    /// This can be called to destroy the device. However, it will only do so if there are no other
    /// references to the device. In debug mode there is some extra tracking/debug info for leaked
    /// device references. If this is not called, destroy will be called when the last device is
    /// dropped.
    pub unsafe fn destroy(&mut self) {
        let mut inner = None;
        std::mem::swap(&mut inner, &mut self.inner);
        let inner = inner.unwrap();
        let strong_count = Arc::strong_count(&inner);
        if let Err(_arc) = inner.try_destroy() {
            log::error!("Could not free the allocator, {} other references exist. Have all allocations been dropped?", strong_count - 1);
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            {
                let mut all_contexts = _arc.all_contexts.lock().unwrap();
                all_contexts.remove(&self.create_index);
                for (k, v) in all_contexts.iter_mut() {
                    v.resolve();
                    println!("context allocation: {}\n{:?}", k, v);
                }
            }
        }
    }
}

impl Drop for VkDeviceContext {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        {
            if let Some(inner) = &self.inner {
                inner
                    .all_contexts
                    .lock()
                    .unwrap()
                    .remove(&self.create_index);
            }
        }
    }
}

/// Represents an error from creating the renderer
#[derive(Debug)]
pub enum VkCreateDeviceError {
    VkError(vk::Result),
    VkMemError(vk_mem::Error),
}

impl std::error::Error for VkCreateDeviceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            VkCreateDeviceError::VkError(ref e) => Some(e),
            VkCreateDeviceError::VkMemError(ref e) => Some(e),
        }
    }
}

impl core::fmt::Display for VkCreateDeviceError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::fmt::Result {
        match *self {
            VkCreateDeviceError::VkError(ref e) => e.fmt(fmt),
            VkCreateDeviceError::VkMemError(ref e) => e.fmt(fmt),
        }
    }
}

impl From<vk::Result> for VkCreateDeviceError {
    fn from(result: vk::Result) -> Self {
        VkCreateDeviceError::VkError(result)
    }
}

impl From<vk_mem::Error> for VkCreateDeviceError {
    fn from(result: vk_mem::Error) -> Self {
        VkCreateDeviceError::VkMemError(result)
    }
}
