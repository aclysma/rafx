use super::internal::*;
use crate::{
    RafxBufferDef, RafxComputePipelineDef, RafxDescriptorSetArrayDef, RafxDeviceContext,
    RafxDeviceInfo, RafxFormat, RafxGraphicsPipelineDef, RafxQueueType, RafxRenderTargetDef,
    RafxResourceType, RafxResult, RafxRootSignatureDef, RafxSampleCount, RafxSamplerDef,
    RafxShaderModuleDefVulkan, RafxShaderStageDef, RafxSwapchainDef, RafxTextureDef,
};
use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;
use raw_window_handle::HasRawWindowHandle;
use std::sync::{Arc, Mutex};

use crate::vulkan::{
    RafxBufferVulkan, RafxDescriptorSetArrayVulkan, RafxFenceVulkan, RafxPipelineVulkan,
    RafxQueueVulkan, RafxRenderTargetVulkan, RafxRootSignatureVulkan, RafxSamplerVulkan,
    RafxSemaphoreVulkan, RafxShaderModuleVulkan, RafxShaderVulkan, RafxSwapchainVulkan,
    RafxTextureVulkan,
};
use ash::extensions::khr;
use fnv::FnvHashMap;
use std::ffi::CStr;
#[cfg(debug_assertions)]
#[cfg(feature = "track-device-contexts")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

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

#[derive(Clone)]
pub struct PhysicalDeviceInfo {
    pub score: i32,
    pub queue_family_indices: VkQueueFamilyIndices,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub extension_properties: Vec<ash::vk::ExtensionProperties>,
    pub all_queue_families: Vec<ash::vk::QueueFamilyProperties>,
}

#[derive(Default, Clone, Debug)]
pub struct VkQueueFamilyIndices {
    pub graphics_queue_family_index: u32,
    pub compute_queue_family_index: u32,
    pub transfer_queue_family_index: u32,
}

pub struct RafxDeviceContextVulkanInner {
    pub(crate) resource_cache: RafxDeviceVulkanResourceCache,
    pub(crate) descriptor_heap: RafxDescriptorHeapVulkan,
    pub(crate) device_info: RafxDeviceInfo,
    pub(crate) queue_allocator: VkQueueAllocatorSet,

    // If we need a dedicated present queue, we share a single queue across all swapchains. This
    // lock ensures that the present operations for those swapchains do not occur concurrently
    pub(crate) dedicated_present_queue_lock: Mutex<()>,

    device: ash::Device,
    allocator: vk_mem::Allocator,
    destroyed: AtomicBool,
    entry: Arc<VkEntry>,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    physical_device_info: PhysicalDeviceInfo,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    next_create_index: AtomicU64,

    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) all_contexts: Mutex<fnv::FnvHashMap<u64, backtrace::Backtrace>>,
}

impl Drop for RafxDeviceContextVulkanInner {
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

impl RafxDeviceContextVulkanInner {
    pub fn new(instance: &VkInstance) -> RafxResult<Self> {
        let physical_device_type_priority = vec![
            PhysicalDeviceType::DiscreteGpu,
            PhysicalDeviceType::IntegratedGpu,
        ];

        // Pick a physical device
        let (physical_device, physical_device_info) =
            choose_physical_device(&instance.instance, &physical_device_type_priority)?;

        //TODO: Don't hardcode queue counts
        let queue_requirements = VkQueueRequirements::determine_required_queue_counts(
            physical_device_info.queue_family_indices.clone(),
            &physical_device_info.all_queue_families,
            VkQueueAllocationStrategy::ShareFirstQueueInFamily,
            VkQueueAllocationStrategy::ShareFirstQueueInFamily,
            VkQueueAllocationStrategy::ShareFirstQueueInFamily,
        );

        // Create a logical device
        let logical_device = create_logical_device(
            &instance.instance,
            physical_device,
            &physical_device_info,
            &queue_requirements,
        )?;

        let queue_allocator = VkQueueAllocatorSet::new(
            &logical_device,
            &physical_device_info.all_queue_families,
            queue_requirements,
        );

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

        let limits = &physical_device_info.properties.limits;

        let device_info = RafxDeviceInfo {
            min_uniform_buffer_offset_alignment: limits.min_uniform_buffer_offset_alignment as u32,
            min_storage_buffer_offset_alignment: limits.min_storage_buffer_offset_alignment as u32,
            upload_buffer_texture_alignment: limits.optimal_buffer_copy_offset_alignment as u32,
            upload_buffer_texture_row_alignment: limits.optimal_buffer_copy_row_pitch_alignment
                as u32,
            supports_clamp_to_border_color: true,
        };

        let resource_cache = RafxDeviceVulkanResourceCache::default();
        let descriptor_heap = RafxDescriptorHeapVulkan::new(&logical_device)?;

        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let all_contexts = {
            let create_backtrace = backtrace::Backtrace::new_unresolved();
            let mut all_contexts = fnv::FnvHashMap::<u64, backtrace::Backtrace>::default();
            all_contexts.insert(0, create_backtrace);
            all_contexts
        };

        Ok(RafxDeviceContextVulkanInner {
            resource_cache,
            descriptor_heap,
            device_info,
            queue_allocator,
            dedicated_present_queue_lock: Mutex::default(),
            entry: instance.entry.clone(),
            instance: instance.instance.clone(),
            physical_device,
            physical_device_info,
            device: logical_device,
            allocator,
            destroyed: AtomicBool::new(false),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            all_contexts: Mutex::new(all_contexts),

            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            next_create_index: AtomicU64::new(1),
        })
    }
}

pub struct RafxDeviceContextVulkan {
    pub(crate) inner: Arc<RafxDeviceContextVulkanInner>,
    #[cfg(debug_assertions)]
    #[cfg(feature = "track-device-contexts")]
    pub(crate) create_index: u64,
}

impl std::fmt::Debug for RafxDeviceContextVulkan {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct("RafxDeviceContextVulkan")
            .field("handle", &self.device().handle())
            .finish()
    }
}

impl Clone for RafxDeviceContextVulkan {
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        let create_index = {
            let create_index = self.inner.next_create_index.fetch_add(1, Ordering::Relaxed);

            #[cfg(feature = "track-device-contexts")]
            {
                let create_backtrace = backtrace::Backtrace::new_unresolved();
                self.inner
                    .as_ref()
                    .all_contexts
                    .lock()
                    .unwrap()
                    .insert(create_index, create_backtrace);
            }

            log::trace!(
                "Cloned RafxDeviceContextVulkan create_index {}",
                create_index
            );
            create_index
        };
        RafxDeviceContextVulkan {
            inner: self.inner.clone(),
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index,
        }
    }
}

impl Drop for RafxDeviceContextVulkan {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        #[cfg(feature = "track-device-contexts")]
        {
            self.inner
                .all_contexts
                .lock()
                .unwrap()
                .remove(&self.create_index);
        }
    }
}

impl Into<RafxDeviceContext> for RafxDeviceContextVulkan {
    fn into(self) -> RafxDeviceContext {
        RafxDeviceContext::Vk(self)
    }
}

impl RafxDeviceContextVulkan {
    pub(crate) fn resource_cache(&self) -> &RafxDeviceVulkanResourceCache {
        &self.inner.resource_cache
    }

    pub(crate) fn descriptor_heap(&self) -> &RafxDescriptorHeapVulkan {
        &self.inner.descriptor_heap
    }

    pub fn device_info(&self) -> &RafxDeviceInfo {
        &self.inner.device_info
    }

    pub fn entry(&self) -> &VkEntry {
        &*self.inner.entry
    }

    pub fn instance(&self) -> &ash::Instance {
        &self.inner.instance
    }

    pub fn device(&self) -> &ash::Device {
        &self.inner.device
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.inner.physical_device
    }

    pub fn physical_device_info(&self) -> &PhysicalDeviceInfo {
        &self.inner.physical_device_info
    }

    pub fn limits(&self) -> &vk::PhysicalDeviceLimits {
        &self.physical_device_info().properties.limits
    }

    pub fn allocator(&self) -> &vk_mem::Allocator {
        &self.inner.allocator
    }

    pub fn queue_allocator(&self) -> &VkQueueAllocatorSet {
        &self.inner.queue_allocator
    }

    pub fn queue_family_indices(&self) -> &VkQueueFamilyIndices {
        &self.inner.physical_device_info.queue_family_indices
    }

    pub fn dedicated_present_queue_lock(&self) -> &Mutex<()> {
        &self.inner.dedicated_present_queue_lock
    }

    pub fn new(
        // instance: &VkInstance,
        // window: &dyn HasRawWindowHandle,
        inner: Arc<RafxDeviceContextVulkanInner>,
    ) -> RafxResult<Self> {
        //let inner = RafxDeviceContextVulkanInner::new(instance, window)?;

        Ok(RafxDeviceContextVulkan {
            inner,
            #[cfg(debug_assertions)]
            #[cfg(feature = "track-device-contexts")]
            create_index: 0,
        })
    }

    pub fn create_queue(
        &self,
        queue_type: RafxQueueType,
    ) -> RafxResult<RafxQueueVulkan> {
        RafxQueueVulkan::new(self, queue_type)
    }

    pub fn create_fence(&self) -> RafxResult<RafxFenceVulkan> {
        RafxFenceVulkan::new(self)
    }

    pub fn create_semaphore(&self) -> RafxResult<RafxSemaphoreVulkan> {
        RafxSemaphoreVulkan::new(self)
    }

    pub fn create_swapchain(
        &self,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainVulkan> {
        RafxSwapchainVulkan::new(self, raw_window_handle, swapchain_def)
    }

    pub fn wait_for_fences(
        &self,
        fences: &[&RafxFenceVulkan],
    ) -> RafxResult<()> {
        RafxFenceVulkan::wait_for_fences(self, fences)
    }

    pub fn create_sampler(
        &self,
        sampler_def: &RafxSamplerDef,
    ) -> RafxResult<RafxSamplerVulkan> {
        RafxSamplerVulkan::new(self, sampler_def)
    }

    pub fn create_texture(
        &self,
        texture_def: &RafxTextureDef,
    ) -> RafxResult<RafxTextureVulkan> {
        RafxTextureVulkan::new(self, texture_def)
    }

    pub fn create_render_target(
        &self,
        render_target_def: &RafxRenderTargetDef,
    ) -> RafxResult<RafxRenderTargetVulkan> {
        RafxRenderTargetVulkan::new(self, render_target_def)
    }

    pub fn create_buffer(
        &self,
        buffer_def: &RafxBufferDef,
    ) -> RafxResult<RafxBufferVulkan> {
        RafxBufferVulkan::new(self, buffer_def)
    }

    pub fn create_shader(
        &self,
        stages: Vec<RafxShaderStageDef>,
    ) -> RafxResult<RafxShaderVulkan> {
        RafxShaderVulkan::new(self, stages)
    }

    pub fn create_root_signature(
        &self,
        root_signature_def: &RafxRootSignatureDef,
    ) -> RafxResult<RafxRootSignatureVulkan> {
        RafxRootSignatureVulkan::new(self, root_signature_def)
    }

    pub fn create_descriptor_set_array(
        &self,
        descriptor_set_array_def: &RafxDescriptorSetArrayDef,
    ) -> RafxResult<RafxDescriptorSetArrayVulkan> {
        RafxDescriptorSetArrayVulkan::new(self, self.descriptor_heap(), descriptor_set_array_def)
    }

    pub fn create_graphics_pipeline(
        &self,
        graphics_pipeline_def: &RafxGraphicsPipelineDef,
    ) -> RafxResult<RafxPipelineVulkan> {
        RafxPipelineVulkan::new_graphics_pipeline(self, graphics_pipeline_def)
    }

    pub fn create_compute_pipeline(
        &self,
        compute_pipeline_def: &RafxComputePipelineDef,
    ) -> RafxResult<RafxPipelineVulkan> {
        RafxPipelineVulkan::new_compute_pipeline(self, compute_pipeline_def)
    }

    pub(crate) fn create_renderpass(
        &self,
        renderpass_def: &RafxRenderpassVulkanDef,
    ) -> RafxResult<RafxRenderpassVulkan> {
        RafxRenderpassVulkan::new(self, renderpass_def)
    }

    pub fn create_shader_module(
        &self,
        data: RafxShaderModuleDefVulkan,
    ) -> RafxResult<RafxShaderModuleVulkan> {
        RafxShaderModuleVulkan::new(self, data)
    }

    // // Just expects bytes with no particular alignment requirements, suitable for reading from a file
    // pub fn create_shader_module_from_bytes(
    //     &self,
    //     data: &[u8],
    // ) -> RafxResult<RafxShaderModuleVulkan> {
    //     RafxShaderModuleVulkan::new_from_bytes(self, data)
    // }
    //
    // // Expects properly aligned, correct endianness, valid SPV
    // pub fn create_shader_module_from_spv(
    //     &self,
    //     spv: &[u32],
    // ) -> RafxResult<RafxShaderModuleVulkan> {
    //     RafxShaderModuleVulkan::new_from_spv(self, spv)
    // }

    pub fn find_supported_format(
        &self,
        candidates: &[RafxFormat],
        resource_type: RafxResourceType,
    ) -> Option<RafxFormat> {
        let mut features = vk::FormatFeatureFlags::empty();
        if resource_type.intersects(RafxResourceType::RENDER_TARGET_COLOR) {
            features |= vk::FormatFeatureFlags::COLOR_ATTACHMENT;
        }

        if resource_type.intersects(RafxResourceType::RENDER_TARGET_DEPTH_STENCIL) {
            features |= vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT;
        }

        do_find_supported_format(
            &self.inner.instance,
            self.inner.physical_device,
            candidates,
            vk::ImageTiling::OPTIMAL,
            features,
        )
    }

    pub fn find_supported_sample_count(
        &self,
        candidates: &[RafxSampleCount],
    ) -> Option<RafxSampleCount> {
        do_find_supported_sample_count(self.limits(), candidates)
    }
}

pub fn do_find_supported_format(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    candidates: &[RafxFormat],
    image_tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Option<RafxFormat> {
    for &candidate in candidates {
        let props = unsafe {
            instance.get_physical_device_format_properties(physical_device, candidate.into())
        };

        let is_supported = match image_tiling {
            vk::ImageTiling::LINEAR => (props.linear_tiling_features & features) == features,
            vk::ImageTiling::OPTIMAL => (props.optimal_tiling_features & features) == features,
            _ => unimplemented!(),
        };

        if is_supported {
            return Some(candidate);
        }
    }

    None
}

fn do_find_supported_sample_count(
    limits: &vk::PhysicalDeviceLimits,
    sample_count_priority: &[RafxSampleCount],
) -> Option<RafxSampleCount> {
    for &sample_count in sample_count_priority {
        let vk_sample_count: vk::SampleCountFlags = sample_count.into();
        if (vk_sample_count.as_raw()
            & limits.framebuffer_depth_sample_counts.as_raw()
            & limits.framebuffer_color_sample_counts.as_raw())
            != 0
        {
            log::trace!("Sample count {:?} is supported", sample_count);
            return Some(sample_count);
        } else {
            log::trace!("Sample count {:?} is unsupported", sample_count);
        }
    }

    None
}

fn choose_physical_device(
    instance: &ash::Instance,
    physical_device_type_priority: &[PhysicalDeviceType],
) -> RafxResult<(ash::vk::PhysicalDevice, PhysicalDeviceInfo)> {
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    if physical_devices.is_empty() {
        panic!("Could not find a physical device");
    }

    let mut best_physical_device = None;
    let mut best_physical_device_info = None;
    let mut best_physical_device_score = -1;

    // let mut best_physical_device_queue_family_indices = None;
    for physical_device in physical_devices {
        let result = query_physical_device_info(
            instance,
            physical_device,
            //surface_loader,
            //surface,
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
    //surface_loader: &ash::extensions::khr::Surface,
    //surface: ash::vk::SurfaceKHR,
    physical_device_type_priority: &[PhysicalDeviceType],
) -> RafxResult<Option<PhysicalDeviceInfo>> {
    log::info!(
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
    let all_queue_families: Vec<ash::vk::QueueFamilyProperties> =
        unsafe { instance.get_physical_device_queue_family_properties(device) };

    let queue_family_indices = find_queue_families(&all_queue_families)?;
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

        log::info!(
            "Found suitable device '{}' API: {} DriverVersion: {} Score = {}",
            device_name,
            vk_version_to_string(properties.api_version),
            vk_version_to_string(properties.driver_version),
            score
        );

        let result = PhysicalDeviceInfo {
            score,
            queue_family_indices,
            properties,
            extension_properties: extensions,
            features,
            all_queue_families,
        };

        log::trace!("{:#?}", properties);
        Ok(Some(result))
    } else {
        log::info!(
            "Found unsuitable device '{}' API: {} DriverVersion: {} could not find queue families",
            device_name,
            vk_version_to_string(properties.api_version),
            vk_version_to_string(properties.driver_version)
        );
        log::trace!("{:#?}", properties);
        Ok(None)
    }
}

//TODO: Could improve this by looking at vendor/device ID, VRAM size, supported feature set, etc.
fn find_queue_families(
    all_queue_families: &[ash::vk::QueueFamilyProperties]
) -> RafxResult<Option<VkQueueFamilyIndices>> {
    let mut graphics_queue_family_index = None;
    let mut compute_queue_family_index = None;
    let mut transfer_queue_family_index = None;

    log::info!("Available queue families:");
    for (queue_family_index, queue_family) in all_queue_families.iter().enumerate() {
        log::info!("Queue Family {}", queue_family_index);
        log::info!("{:#?}", queue_family);
    }

    //
    // Find the first queue family that supports graphics and use it for graphics
    //
    for (queue_family_index, queue_family) in all_queue_families.iter().enumerate() {
        let queue_family_index = queue_family_index as u32;
        let supports_graphics = queue_family.queue_flags & ash::vk::QueueFlags::GRAPHICS
            == ash::vk::QueueFlags::GRAPHICS;

        if supports_graphics {
            graphics_queue_family_index = Some(queue_family_index);
            break;
        }
    }

    //
    // Find a compute queue family in the following order of preference:
    // - Doesn't support graphics
    // - Supports graphics but hasn't already been claimed by graphics
    // - Fallback to using the graphics queue family as it's guaranteed to support compute
    //
    for (queue_family_index, queue_family) in all_queue_families.iter().enumerate() {
        let queue_family_index = queue_family_index as u32;
        let supports_graphics = queue_family.queue_flags & ash::vk::QueueFlags::GRAPHICS
            == ash::vk::QueueFlags::GRAPHICS;
        let supports_compute =
            queue_family.queue_flags & ash::vk::QueueFlags::COMPUTE == ash::vk::QueueFlags::COMPUTE;

        if !supports_graphics && supports_compute {
            // Ideally we want to find a dedicated compute queue (i.e. doesn't support graphics)
            compute_queue_family_index = Some(queue_family_index);
            break;
        } else if supports_compute
            && compute_queue_family_index.is_none()
            && Some(queue_family_index) != graphics_queue_family_index
        {
            // Otherwise accept the first queue that supports compute that is NOT the graphics queue
            compute_queue_family_index = Some(queue_family_index);
        }
    }

    // If we didn't find a compute queue family != graphics queue family, settle for using the
    // graphics queue family. It's guaranteed to support compute.
    if compute_queue_family_index.is_none() {
        compute_queue_family_index = graphics_queue_family_index;
    }

    //
    // Find a transfer queue family in the following order of preference:
    // - Doesn't support graphics or compute
    // - Supports graphics but hasn't already been claimed by compute or graphics
    // - Fallback to using the graphics queue family as it's guaranteed to support transfers
    //
    for (queue_family_index, queue_family) in all_queue_families.iter().enumerate() {
        let queue_family_index = queue_family_index as u32;
        let supports_graphics = queue_family.queue_flags & ash::vk::QueueFlags::GRAPHICS
            == ash::vk::QueueFlags::GRAPHICS;
        let supports_compute =
            queue_family.queue_flags & ash::vk::QueueFlags::COMPUTE == ash::vk::QueueFlags::COMPUTE;
        let supports_transfer = queue_family.queue_flags & ash::vk::QueueFlags::TRANSFER
            == ash::vk::QueueFlags::TRANSFER;

        if !supports_graphics && !supports_compute && supports_transfer {
            // Ideally we want to find a dedicated transfer queue
            transfer_queue_family_index = Some(queue_family_index);
            break;
        } else if supports_transfer
            && transfer_queue_family_index.is_none()
            && Some(queue_family_index) != graphics_queue_family_index
            && Some(queue_family_index) != compute_queue_family_index
        {
            // Otherwise accept the first queue that supports transfers that is NOT the graphics queue or compute queue
            transfer_queue_family_index = Some(queue_family_index);
        }
    }

    // If we didn't find a transfer queue family != graphics queue family, settle for using the
    // graphics queue family. It's guaranteed to support transfer.
    if transfer_queue_family_index.is_none() {
        transfer_queue_family_index = graphics_queue_family_index;
    }

    log::info!(
        "Graphics QF: {:?}  Compute QF: {:?}  Transfer QF: {:?}",
        graphics_queue_family_index,
        compute_queue_family_index,
        transfer_queue_family_index
    );

    if let (
        Some(graphics_queue_family_index),
        Some(compute_queue_family_index),
        Some(transfer_queue_family_index),
    ) = (
        graphics_queue_family_index,
        compute_queue_family_index,
        transfer_queue_family_index,
    ) {
        Ok(Some(VkQueueFamilyIndices {
            graphics_queue_family_index,
            compute_queue_family_index,
            transfer_queue_family_index,
        }))
    } else {
        Ok(None)
    }
}

fn create_logical_device(
    instance: &ash::Instance,
    physical_device: ash::vk::PhysicalDevice,
    physical_device_info: &PhysicalDeviceInfo,
    queue_requirements: &VkQueueRequirements,
) -> RafxResult<ash::Device> {
    //TODO: Ideally we would set up validation layers for the logical device too.

    fn khr_portability_subset_extension_name() -> &'static CStr {
        CStr::from_bytes_with_nul(b"VK_KHR_portability_subset\0").expect("Wrong extension string")
    }

    let mut device_extension_names = vec![khr::Swapchain::name().as_ptr()];

    // Add VK_KHR_portability_subset if the extension exists (this is mandated by spec)
    let portability_subset_extension_name = khr_portability_subset_extension_name();
    for extension in &physical_device_info.extension_properties {
        let extension_name = unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) };

        if extension_name == portability_subset_extension_name {
            device_extension_names.push(khr_portability_subset_extension_name().as_ptr());
            break;
        }
    }

    // Features enabled here by default are supported very widely (only unsupported devices on
    // vulkan.gpuinfo.org are SwiftShader, a software renderer.
    let features = vk::PhysicalDeviceFeatures::builder()
        .sampler_anisotropy(true)
        .sample_rate_shading(true)
        // Used for debug drawing lines/points
        .fill_mode_non_solid(true);

    let mut queue_families_to_create = FnvHashMap::default();
    for (&queue_family_index, &count) in &queue_requirements.queue_counts {
        queue_families_to_create.insert(queue_family_index, vec![1.0 as f32; count as usize]);
    }

    let queue_infos: Vec<_> = queue_families_to_create
        .iter()
        .map(|(&queue_family_index, priorities)| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(priorities)
                .build()
        })
        .collect();

    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extension_names)
        .enabled_features(&features);

    let device: ash::Device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };

    Ok(device)
}
