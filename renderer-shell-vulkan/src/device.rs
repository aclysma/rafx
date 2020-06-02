use ash::vk;
use ash::prelude::VkResult;
use super::VkInstance;

use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;
use super::Window;

use std::ffi::CStr;

use ash::extensions::khr;
use crate::{PhysicalDeviceType /*, VkSubmitQueue*/};
use std::mem::ManuallyDrop;

use std::sync::{Arc, Mutex};

pub struct VkQueue {
    pub queue_family_index: u32,
    pub queue: vk::Queue,
}

/// Has the indexes for all the queue families we will need. It's possible a single family
/// is used for both graphics and presentation, in which case the index will be the same
#[derive(Default, Clone)]
pub struct VkQueueFamilyIndices {
    pub transfer_queue_family_index: u32,
    pub graphics_queue_family_index: u32,
    pub present_queue_family_index: u32,
}

/// An instantiated queue per queue family. We only need one queue per family.
#[derive(Clone)]
pub struct VkQueues {
    pub transfer_queue: ash::vk::Queue,
    pub graphics_queue: ash::vk::Queue,
    pub present_queue: ash::vk::Queue,
}

#[derive(Clone)]
pub struct PhysicalDeviceInfo {
    pub score: i32,
    pub queue_family_indices: VkQueueFamilyIndices,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub extension_properties: Vec<ash::vk::ExtensionProperties>,
}

pub struct VkDeviceContextInner {
    instance: ash::Instance,
    device: ash::Device,
    allocator: vk_mem::Allocator,
    surface: vk::SurfaceKHR,
    surface_loader: ash::extensions::khr::Surface,
    physical_device: vk::PhysicalDevice,
    physical_device_info: PhysicalDeviceInfo,
    queues: VkQueues,
}

/// A lighter-weight structure that can be cached on downstream users. It includes
/// access to vk::Device and allocators.
#[derive(Clone)]
pub struct VkDeviceContext {
    inner: Option<Arc<ManuallyDrop<VkDeviceContextInner>>>,
}

impl VkDeviceContext {
    pub fn instance(&self) -> &ash::Instance {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .instance
    }

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

    pub fn surface(&self) -> vk::SurfaceKHR {
        self.inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .surface
    }

    pub fn surface_loader(&self) -> &ash::extensions::khr::Surface {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .surface_loader
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .physical_device
    }

    pub fn physical_device_info(&self) -> &PhysicalDeviceInfo {
        &self.inner
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

    pub fn queues(&self) -> &VkQueues {
        &self
            .inner
            .as_ref()
            .expect("inner is only None if VkDevice is dropped")
            .queues
    }

    fn new(
        instance: ash::Instance,
        device: ash::Device,
        allocator: vk_mem::Allocator,
        surface: ash::vk::SurfaceKHR,
        surface_loader: ash::extensions::khr::Surface,
        physical_device: ash::vk::PhysicalDevice,
        physical_device_info: PhysicalDeviceInfo,
        queues: VkQueues,
    ) -> Self {
        VkDeviceContext {
            inner: Some(Arc::new(ManuallyDrop::new(VkDeviceContextInner {
                instance,
                device,
                allocator,
                surface,
                surface_loader,
                physical_device,
                physical_device_info,
                queues,
            }))),
        }
    }

    unsafe fn destroy(&mut self) {
        let mut inner = None;
        std::mem::swap(&mut inner, &mut self.inner);
        if let Ok(mut inner) = Arc::try_unwrap(inner.unwrap()) {
            inner.allocator.destroy();
            inner.device.destroy_device(None);
            ManuallyDrop::drop(&mut inner);
        } else {
            panic!("Could not free the allocator, something is holding a reference to it. Have all allocations been dropped?")
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

/// Represents the instance and device. Most of the code here has to do with picking a good device
/// that's compatible with the window we're given. The VkDevice is the "heavy-weight" structure
/// that will destroy all vulkan resources when it's dropped. VkDeviceContext is a lighter-weight
/// structure that should generally be used instead. It is expected that all VkDeviceContext
/// structures based on this VkDevice are destroyed before dropping the VkDevice.
pub struct VkDevice {
    pub device_context: VkDeviceContext,
    pub surface: ash::vk::SurfaceKHR,
    pub surface_loader: ash::extensions::khr::Surface,
    pub physical_device: ash::vk::PhysicalDevice,
    pub physical_device_info: PhysicalDeviceInfo,
    //pub queue_family_indices: VkQueueFamilyIndices,
    pub queues: VkQueues,
    //pub memory_properties: vk::PhysicalDeviceMemoryProperties,
}

impl VkDevice {
    pub fn allocator(&self) -> &vk_mem::Allocator {
        self.device_context.allocator()
    }

    pub fn device(&self) -> &ash::Device {
        self.device_context.device()
    }

    pub fn new(
        instance: &VkInstance,
        window: &dyn Window,
        physical_device_type_priority: &[PhysicalDeviceType],
    ) -> Result<Self, VkCreateDeviceError> {
        // Get the surface, needed to select the best queue family
        let surface = window
            .create_vulkan_surface(&instance.entry, &instance.instance)
            .expect("Could not create vulkan surface");

        let surface_loader = khr::Surface::new(&instance.entry, &instance.instance);

        // Pick a physical device
        let (physical_device, physical_device_info) = Self::choose_physical_device(
            &instance.instance,
            &surface_loader,
            surface,
            physical_device_type_priority,
        )?;

        // Create a logical device
        let (logical_device, queues) = Self::create_logical_device(
            &instance.instance,
            physical_device,
            &physical_device_info.queue_family_indices,
        )?;

        let allocator_create_info = vk_mem::AllocatorCreateInfo {
            physical_device,
            device: logical_device.clone(),
            instance: instance.instance.clone(),
            flags: vk_mem::AllocatorCreateFlags::default(),
            preferred_large_heap_block_size: Default::default(),
            frame_in_use_count: 0, // Not using CAN_BECOME_LOST, so this is not needed
            heap_size_limits: Default::default(),
        };

        let allocator = vk_mem::Allocator::new(&allocator_create_info)?;

        let memory_properties = unsafe {
            instance
                .instance
                .get_physical_device_memory_properties(physical_device)
        };

        let device_context = VkDeviceContext::new(
            instance.instance.clone(),
            logical_device,
            allocator,
            surface,
            surface_loader.clone(),
            physical_device,
            physical_device_info.clone(),
            queues.clone(),
        );

        Ok(VkDevice {
            device_context,
            surface,
            surface_loader,
            physical_device,
            physical_device_info,
            queues,
        })
    }

    fn choose_physical_device(
        instance: &ash::Instance,
        surface_loader: &ash::extensions::khr::Surface,
        surface: ash::vk::SurfaceKHR,
        physical_device_type_priority: &[PhysicalDeviceType],
    ) -> VkResult<(ash::vk::PhysicalDevice, PhysicalDeviceInfo)> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };

        if physical_devices.is_empty() {
            panic!("Could not find a physical device");
        }

        let mut best_physical_device = None;
        let mut best_physical_device_info = None;
        let mut best_physical_device_score = -1;
        // let mut best_physical_device_queue_family_indices = None;
        for physical_device in physical_devices {
            let result = Self::query_physical_device_info(
                instance,
                physical_device,
                surface_loader,
                surface,
                physical_device_type_priority,
            );

            if let Some(physical_device_info) = result? {
                if physical_device_info.score > best_physical_device_score {
                    best_physical_device = Some(physical_device);
                    best_physical_device_score = physical_device_info.score;
                    best_physical_device_info = Some(physical_device_info);

                }
            }
        }

        //TODO: Return an error
        let physical_device = best_physical_device.expect("Could not find suitable device");
        let physical_device_info = best_physical_device_info.unwrap();

        Ok((physical_device, physical_device_info))
    }

    fn vk_version_to_string(version: u32) -> String {
        format!(
            "{}.{}.{}",
            vk::version_major(version),
            vk::version_minor(version),
            vk::version_patch(version)
        )
    }

    fn query_physical_device_info(
        instance: &ash::Instance,
        device: ash::vk::PhysicalDevice,
        surface_loader: &ash::extensions::khr::Surface,
        surface: ash::vk::SurfaceKHR,
        physical_device_type_priority: &[PhysicalDeviceType],
    ) -> VkResult<Option<PhysicalDeviceInfo>> {
        info!(
            "Preferred device types: {:?}",
            physical_device_type_priority
        );

        let properties: ash::vk::PhysicalDeviceProperties =
            unsafe { instance.get_physical_device_properties(device) };
        let device_name = unsafe {
            CStr::from_ptr(properties.device_name.as_ptr())
                .to_str()
                .unwrap()
                .to_string()
        };

        //TODO: Check that the extensions we want to use are supported
        let extensions: Vec<ash::vk::ExtensionProperties> =
            unsafe { instance.enumerate_device_extension_properties(device)? };
        let features: vk::PhysicalDeviceFeatures =
            unsafe { instance.get_physical_device_features(device) };

        let queue_family_indices =
            Self::find_queue_families(instance, device, surface_loader, surface)?;
        if let Some(queue_family_indices) = queue_family_indices {
            // Determine the index of the device_type within physical_device_type_priority
            let index = physical_device_type_priority
                .iter()
                .map(|x| x.to_vk())
                .position(|x| x == properties.device_type);

            // Convert it to a score
            let rank = if let Some(index) = index {
                // It's in the list, return a value between 1..n
                physical_device_type_priority.len() - index
            } else {
                // Not in the list, return a zero
                0
            } as i32;

            let mut score = 0;
            score += rank * 100;

            info!(
                "Found suitable device '{}' API: {} DriverVersion: {} Score = {}",
                device_name,
                Self::vk_version_to_string(properties.api_version),
                Self::vk_version_to_string(properties.driver_version),
                score
            );

            let result = PhysicalDeviceInfo {
                score,
                queue_family_indices,
                properties,
                extension_properties: extensions,
                features
            };

            trace!("{:#?}", properties);
            Ok(Some(result))
        } else {
            info!(
                "Found unsuitable device '{}' API: {} DriverVersion: {} could not find queue families",
                device_name,
                Self::vk_version_to_string(properties.api_version),
                Self::vk_version_to_string(properties.driver_version)
            );
            trace!("{:#?}", properties);
            Ok(None)
        }
    }

    fn find_queue_families(
        instance: &ash::Instance,
        physical_device: ash::vk::PhysicalDevice,
        surface_loader: &ash::extensions::khr::Surface,
        surface: ash::vk::SurfaceKHR,
    ) -> VkResult<Option<VkQueueFamilyIndices>> {
        let queue_families: Vec<ash::vk::QueueFamilyProperties> =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut graphics_queue_family_index = None;
        let mut present_queue_family_index = None;
        let mut transfer_queue_family_index = None;
        let mut transfer_queue_family_is_dedicated = false;

        info!("Available queue families:");
        for (queue_family_index, queue_family) in queue_families.iter().enumerate() {
            info!("Queue Family {}", queue_family_index);
            info!("{:#?}", queue_family);
        }

        for (queue_family_index, queue_family) in queue_families.iter().enumerate() {
            let queue_family_index = queue_family_index as u32;

            let supports_graphics = queue_family.queue_flags & ash::vk::QueueFlags::GRAPHICS
                == ash::vk::QueueFlags::GRAPHICS;
            let supports_transfer = queue_family.queue_flags & ash::vk::QueueFlags::TRANSFER
                == ash::vk::QueueFlags::TRANSFER;
            let supports_present = unsafe {
                surface_loader.get_physical_device_surface_support(
                    physical_device,
                    queue_family_index,
                    surface,
                )?
            };

            // Remember the first graphics queue family we saw...
            if supports_graphics && graphics_queue_family_index.is_none() {
                graphics_queue_family_index = Some(queue_family_index);
            }

            // and the first present queue family we saw
            if supports_present && present_queue_family_index.is_none() {
                present_queue_family_index = Some(queue_family_index);
            }

            // A queue family that supports both is ideal, use that instead if we find it
            if supports_graphics && supports_present {
                // Use the first queue family that supports both
                if graphics_queue_family_index != present_queue_family_index {
                    graphics_queue_family_index = Some(queue_family_index);
                    present_queue_family_index = Some(queue_family_index);
                }
            }

            if !supports_graphics && supports_transfer && !transfer_queue_family_is_dedicated {
                // Ideally we want to find a dedicated transfer queue
                transfer_queue_family_index = Some(queue_family_index);
                transfer_queue_family_is_dedicated = true;
            } else if supports_transfer
                && transfer_queue_family_index.is_none()
                && Some(queue_family_index) != graphics_queue_family_index
            {
                // Otherwise accept the first queue that supports transfers that is NOT the graphics queue
                transfer_queue_family_index = Some(queue_family_index);
            }
        }

        // If we didn't find a transfer queue family != graphics queue family, settle for using the
        // graphics queue family. It's guaranteed to support transfer.
        if transfer_queue_family_index.is_none() {
            transfer_queue_family_index = graphics_queue_family_index;
        }

        info!(
            "Graphics QF: {:?}  Present QF: {:?}  Transfer QF: {:?}",
            graphics_queue_family_index, present_queue_family_index, transfer_queue_family_index
        );

        if let (
            Some(graphics_queue_family_index),
            Some(present_queue_family_index),
            Some(transfer_queue_family_index),
        ) = (
            graphics_queue_family_index,
            present_queue_family_index,
            transfer_queue_family_index,
        ) {
            Ok(Some(VkQueueFamilyIndices {
                graphics_queue_family_index,
                present_queue_family_index,
                transfer_queue_family_index,
            }))
        } else {
            Ok(None)
        }
    }

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: ash::vk::PhysicalDevice,
        queue_family_indices: &VkQueueFamilyIndices,
    ) -> VkResult<(ash::Device, VkQueues)> {
        //TODO: Ideally we would set up validation layers for the logical device too.

        let device_extension_names_raw = [khr::Swapchain::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures::builder()
            .fill_mode_non_solid(true);
        let priorities = [1.0];

        let mut queue_families_to_create = std::collections::HashSet::new();
        queue_families_to_create.insert(queue_family_indices.graphics_queue_family_index);
        queue_families_to_create.insert(queue_family_indices.present_queue_family_index);
        queue_families_to_create.insert(queue_family_indices.transfer_queue_family_index);

        let queue_infos: Vec<_> = queue_families_to_create
            .iter()
            .map(|queue_family_index| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*queue_family_index)
                    .queue_priorities(&priorities)
                    .build()
            })
            .collect();

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: ash::Device =
            unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        let graphics_queue =
            unsafe { device.get_device_queue(queue_family_indices.graphics_queue_family_index, 0) };

        let present_queue =
            unsafe { device.get_device_queue(queue_family_indices.present_queue_family_index, 0) };

        let transfer_queue =
            unsafe { device.get_device_queue(queue_family_indices.transfer_queue_family_index, 0) };

        let queues = VkQueues {
            graphics_queue,
            present_queue,
            transfer_queue,
        };

        Ok((device, queues))
    }
}

impl Drop for VkDevice {
    fn drop(&mut self) {
        trace!("destroying VkDevice");
        unsafe {
            self.device_context.destroy();
            self.surface_loader.destroy_surface(self.surface, None);
        }

        trace!("destroyed VkDevice");
    }
}
