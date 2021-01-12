use crate::vulkan::{
    RafxDeviceContextVulkan, RafxFenceVulkan, RafxRawImageVulkan, RafxRenderTargetVulkan,
    RafxSemaphoreVulkan, VkEntry,
};
use crate::{
    RafxCommandBufferDef, RafxCommandPoolDef, RafxError, RafxExtents3D, RafxFormat, RafxQueueType,
    RafxRenderTargetBarrier, RafxRenderTargetDef, RafxResourceState, RafxResourceType, RafxResult,
    RafxSampleCount, RafxSwapchainDef, RafxSwapchainImage, RafxTextureDimensions,
};
use ash::version::DeviceV1_0;
use ash::vk;
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

//TODO: Allow these to be overridden when setting up vulkan?
const VSYNC_ON_PRESENT_MODES: [VkPresentMode; 2] = [VkPresentMode::Mailbox, VkPresentMode::Fifo];
const VSYNC_OFF_PRESENT_MODES: [VkPresentMode; 3] = [
    VkPresentMode::FifoRelaxed,
    VkPresentMode::Mailbox,
    VkPresentMode::Fifo,
];

fn present_mode_priority(swapchain_def: &RafxSwapchainDef) -> &'static [VkPresentMode] {
    if swapchain_def.enable_vsync {
        &VSYNC_ON_PRESENT_MODES[..]
    } else {
        &VSYNC_OFF_PRESENT_MODES[..]
    }
}

pub struct RafxSwapchainVulkan {
    device_context: RafxDeviceContextVulkan,
    swapchain: ManuallyDrop<VkSwapchain>,
    swapchain_def: RafxSwapchainDef,
    last_image_suboptimal: bool,
    swapchain_images: Vec<RafxSwapchainImage>,
    surface: vk::SurfaceKHR,
    surface_loader: Arc<khr::Surface>,
}

impl Drop for RafxSwapchainVulkan {
    fn drop(&mut self) {
        log::trace!("destroying RafxSwapchainVulkan");

        unsafe {
            ManuallyDrop::drop(&mut self.swapchain);
            self.surface_loader.destroy_surface(self.surface, None);
        }

        log::trace!("destroyed RafxSwapchainVulkan");
    }
}

impl RafxSwapchainVulkan {
    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainVulkan> {
        // Get the surface, needed to select the best queue family
        let surface = unsafe {
            ash_window::create_surface(
                &*device_context.entry(),
                device_context.instance(),
                raw_window_handle,
                None,
            )?
        };

        let surface_loader = Arc::new(match &device_context.entry() {
            VkEntry::Dynamic(entry) => khr::Surface::new(entry, device_context.instance()),
            #[cfg(feature = "static-vulkan")]
            VkEntry::Static(entry) => khr::Surface::new(entry, &instance.instance),
        });

        let present_mode_priority = present_mode_priority(swapchain_def);

        let swapchain = VkSwapchain::new(
            device_context,
            surface,
            &surface_loader,
            None,
            present_mode_priority,
            vk::Extent2D {
                width: swapchain_def.width,
                height: swapchain_def.height,
            },
        )
        .map_err(|e| format!("{:?}", e))?;

        //TODO: Check image count of swapchain and update swapchain_def with swapchain.swapchain_images.len();
        let swapchain_def = swapchain_def.clone();

        let swapchain_images = Self::setup_swapchain_images(device_context, &swapchain)?;

        Ok(RafxSwapchainVulkan {
            device_context: device_context.clone(),
            swapchain: ManuallyDrop::new(swapchain),
            swapchain_def,
            swapchain_images,
            last_image_suboptimal: false,
            surface,
            surface_loader,
        })
    }

    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        let present_mode_priority = present_mode_priority(swapchain_def);

        let new_swapchain = VkSwapchain::new(
            &self.device_context,
            self.surface,
            &self.surface_loader,
            Some(self.swapchain.swapchain),
            present_mode_priority,
            vk::Extent2D {
                width: swapchain_def.width,
                height: swapchain_def.height,
            },
        )?;

        unsafe {
            ManuallyDrop::drop(&mut self.swapchain);
        }
        self.swapchain = ManuallyDrop::new(new_swapchain);
        self.swapchain_def = swapchain_def.clone();
        self.last_image_suboptimal = false;
        self.swapchain_images =
            Self::setup_swapchain_images(&self.device_context, &self.swapchain)?;
        Ok(())
    }

    pub fn swapchain(&self) -> &VkSwapchain {
        &self.swapchain
    }

    pub fn swapchain_mut(&mut self) -> &mut VkSwapchain {
        &mut self.swapchain
    }

    pub fn swapchain_def(&self) -> &RafxSwapchainDef {
        &self.swapchain_def
    }

    pub fn image_count(&self) -> usize {
        self.swapchain.swapchain_images.len()
    }

    pub fn format(&self) -> RafxFormat {
        self.swapchain.swapchain_info.surface_format.format.into()
    }

    //TODO: Return something like PresentResult?
    pub fn acquire_next_image_fence(
        &mut self,
        fence: &RafxFenceVulkan,
    ) -> RafxResult<RafxSwapchainImage> {
        let result = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                vk::Semaphore::null(),
                fence.vk_fence(),
            )
        };

        match result {
            Ok((present_index, is_suboptimal)) => {
                self.last_image_suboptimal = is_suboptimal;
                fence.set_submitted(true);
                Ok(self.swapchain_images[present_index as usize].clone())
            }
            Err(e) => {
                self.last_image_suboptimal = false;
                unsafe {
                    self.swapchain
                        .device_context
                        .device()
                        .reset_fences(&[fence.vk_fence()])?;
                }
                fence.set_submitted(false);
                Err(RafxError::VkError(e))
            }
        }
    }

    //TODO: Return something like PresentResult?
    pub fn acquire_next_image_semaphore(
        &mut self,
        semaphore: &RafxSemaphoreVulkan,
    ) -> RafxResult<RafxSwapchainImage> {
        let result = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                semaphore.vk_semaphore(),
                vk::Fence::null(),
            )
        };

        match result {
            Ok((present_index, is_suboptimal)) => {
                self.last_image_suboptimal = is_suboptimal;
                semaphore.set_signal_available(true);
                Ok(self.swapchain_images[present_index as usize].clone())
            }
            Err(e) => {
                self.last_image_suboptimal = false;
                semaphore.set_signal_available(false);
                Err(RafxError::VkError(e))
            }
        }
    }

    fn setup_swapchain_images(
        device_context: &RafxDeviceContextVulkan,
        swapchain: &VkSwapchain,
    ) -> RafxResult<Vec<RafxSwapchainImage>> {
        let queue = device_context.create_queue(RafxQueueType::Graphics)?;
        let cmd_pool = queue.create_command_pool(&RafxCommandPoolDef { transient: true })?;
        let command_buffer = cmd_pool.create_command_buffer(&RafxCommandBufferDef {
            is_secondary: false,
        })?;
        command_buffer.begin()?;

        let swapchain_images = swapchain.rafx_images()?;

        let rt_barriers: Vec<_> = swapchain_images
            .iter()
            .map(|image| {
                RafxRenderTargetBarrier::state_transition(
                    &image.render_target,
                    RafxResourceState::UNDEFINED,
                    RafxResourceState::PRESENT,
                )
            })
            .collect();

        command_buffer.cmd_resource_barrier(&[], &[], &rt_barriers)?;

        command_buffer.end()?;
        queue.submit(&[&command_buffer], &[], &[], None)?;
        queue.wait_for_queue_idle()?;
        Ok(swapchain_images)
    }
}

use ash::extensions::khr;
use ash::prelude::VkResult;

use crate::vulkan::device::VkPresentMode;
use ash::vk::Extent2D;
use std::mem::ManuallyDrop;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

struct CreateSwapchainResult {
    swapchain_loader: khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    dedicated_present_queue: Option<vk::Queue>,
}

#[derive(Clone)]
pub struct SwapchainInfo {
    pub surface_format: vk::SurfaceFormatKHR,
    pub present_mode: vk::PresentModeKHR,
    pub extents: vk::Extent2D,
    pub image_count: usize,
    pub image_usage_flags: vk::ImageUsageFlags,
}

/// Handles setting up the swapchain resources required to present
pub struct VkSwapchain {
    //pub device: ash::Device, // VkDevice is responsible for cleaning this up
    pub device_context: RafxDeviceContextVulkan,
    pub surface: vk::SurfaceKHR,
    pub surface_loader: Arc<khr::Surface>,

    pub swapchain_info: SwapchainInfo,
    pub swapchain_loader: Arc<khr::Swapchain>,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,

    pub present_queue_family_index: u32,
    pub dedicated_present_queue: Option<vk::Queue>,
}

impl VkSwapchain {
    pub(crate) fn dedicated_present_queue(&self) -> Option<vk::Queue> {
        self.dedicated_present_queue
    }

    pub fn new(
        device_context: &RafxDeviceContextVulkan,
        surface: vk::SurfaceKHR,
        surface_loader: &Arc<khr::Surface>,
        old_swapchain: Option<vk::SwapchainKHR>,
        present_mode_priority: &[VkPresentMode],
        window_inner_size: Extent2D,
    ) -> VkResult<VkSwapchain> {
        let (available_formats, available_present_modes, surface_capabilities) =
            Self::query_swapchain_support(
                device_context.physical_device(),
                surface,
                &surface_loader,
            )?;

        let surface_format = Self::choose_swapchain_format(&available_formats);
        log::info!("Surface format: {:?}", surface_format);

        let present_mode =
            Self::choose_present_mode(&available_present_modes, present_mode_priority);
        log::info!("Present mode: {:?}", present_mode);

        let extents = Self::choose_extents(&surface_capabilities, window_inner_size);
        log::info!("Extents: {:?}", extents);

        let present_queue_family_index = Self::choose_present_queue_family_index(
            surface,
            &surface_loader,
            device_context.physical_device(),
            &device_context.physical_device_info().all_queue_families,
            device_context
                .queue_family_indices()
                .graphics_queue_family_index,
        )?;

        let swapchain_image_usage_flags = vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let create_swapchain_result = Self::create_swapchain(
            device_context,
            surface,
            &surface_capabilities,
            surface_format,
            extents,
            present_mode,
            swapchain_image_usage_flags,
            old_swapchain,
            present_queue_family_index,
        )?;

        let swapchain_images = unsafe {
            create_swapchain_result
                .swapchain_loader
                .get_swapchain_images(create_swapchain_result.swapchain)?
        };

        let swapchain_info = SwapchainInfo {
            surface_format,
            extents,
            present_mode,
            image_usage_flags: swapchain_image_usage_flags,
            image_count: swapchain_images.len(),
        };

        Ok(VkSwapchain {
            device_context: device_context.clone(),
            surface,
            surface_loader: surface_loader.clone(),
            swapchain_info,
            swapchain_loader: Arc::new(create_swapchain_result.swapchain_loader),
            swapchain: create_swapchain_result.swapchain,
            dedicated_present_queue: create_swapchain_result.dedicated_present_queue,
            swapchain_images,
            present_queue_family_index,
        })
    }

    fn rafx_images(&self) -> RafxResult<Vec<RafxSwapchainImage>> {
        let mut swapchain_images = Vec::with_capacity(self.swapchain_images.len());
        for (image_index, image) in self.swapchain_images.iter().enumerate() {
            let raw_image = RafxRawImageVulkan {
                image: *image,
                allocation: None,
            };

            let render_target = RafxRenderTargetVulkan::from_existing(
                &self.device_context,
                Some(raw_image),
                &RafxRenderTargetDef {
                    extents: RafxExtents3D {
                        width: self.swapchain_info.extents.width,
                        height: self.swapchain_info.extents.height,
                        depth: 1,
                    },
                    array_length: 1,
                    mip_count: 1,
                    format: self.swapchain_info.surface_format.format.into(),
                    resource_type: RafxResourceType::UNDEFINED,
                    //clear_value,
                    sample_count: RafxSampleCount::SampleCount1,
                    //sample_quality
                    dimensions: RafxTextureDimensions::Dim2D,
                },
            )?;

            swapchain_images.push(RafxSwapchainImage {
                render_target: render_target.into(),
                swapchain_image_index: image_index as u32,
            });
        }

        Ok(swapchain_images)
    }

    fn query_swapchain_support(
        physical_device: ash::vk::PhysicalDevice,
        surface: ash::vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
    ) -> VkResult<(
        Vec<vk::SurfaceFormatKHR>,
        Vec<vk::PresentModeKHR>,
        vk::SurfaceCapabilitiesKHR,
    )> {
        let available_formats: Vec<vk::SurfaceFormatKHR> = unsafe {
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?
        };

        let available_present_modes: Vec<vk::PresentModeKHR> = unsafe {
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?
        };

        let surface_capabilities: vk::SurfaceCapabilitiesKHR = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
        };

        Ok((
            available_formats,
            available_present_modes,
            surface_capabilities,
        ))
    }

    fn choose_swapchain_format(available_formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
        let mut best_format = None;

        for available_format in available_formats {
            if available_format.format == ash::vk::Format::B8G8R8A8_SRGB
                && available_format.color_space == ash::vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                best_format = Some(available_format);
            }
        }

        match best_format {
            Some(format) => *format,
            None => available_formats[0],
        }
    }

    fn choose_present_mode(
        available_present_modes: &[vk::PresentModeKHR],
        present_mode_priority: &[VkPresentMode],
    ) -> vk::PresentModeKHR {
        log::info!("Available present modes: {:?}", available_present_modes);
        log::info!("Preferred present modes: {:?}", present_mode_priority);

        let mut best_present_mode = None;

        for present_mode in present_mode_priority.iter().map(|x| x.to_vk()) {
            if available_present_modes.contains(&present_mode) {
                best_present_mode = Some(present_mode);
                break;
            }
        }

        match best_present_mode {
            Some(present_mode) => present_mode,
            None => ash::vk::PresentModeKHR::FIFO, // Per spec, FIFO always exists
        }
    }

    fn choose_extents(
        surface_capabilities: &vk::SurfaceCapabilitiesKHR,
        window_inner_size: Extent2D,
    ) -> ash::vk::Extent2D {
        // Copied from num-traits under MIT/Apache-2.0 dual license. It doesn't make much sense
        // to pull in a whole crate just for this utility function. This will be in std rust soon
        pub fn clamp<T: PartialOrd>(
            input: T,
            min: T,
            max: T,
        ) -> T {
            debug_assert!(min <= max, "min must be less than or equal to max");
            if input < min {
                min
            } else if input > max {
                max
            } else {
                input
            }
        }

        log::trace!(
            "swapchain surface capability min {:?}",
            surface_capabilities.min_image_extent
        );
        log::trace!(
            "swapchain surface capability max {:?}",
            surface_capabilities.max_image_extent
        );
        log::trace!(
            "swapchain surface capability current {:?}",
            surface_capabilities.current_extent
        );

        let mut actual_extent = if surface_capabilities.current_extent.width != std::u32::MAX {
            log::debug!(
                "Swapchain extents chosen by surface capabilities ({} {})",
                surface_capabilities.current_extent.width,
                surface_capabilities.current_extent.height,
            );

            surface_capabilities.current_extent
        } else {
            let actual_extent = ash::vk::Extent2D::builder()
                .width(window_inner_size.width)
                .height(window_inner_size.height)
                .build();

            log::debug!(
                "Swapchain extents chosen by inner window size ({} {})",
                window_inner_size.width,
                window_inner_size.height,
            );

            actual_extent
        };

        // Force x and y >=1 due to spec VUID-VkSwapchainCreateInfoKHR-imageExtent-01689
        // I've seen surface capability return a max size of 0, tripping
        // VUID-VkSwapchainCreateInfoKHR-imageExtent-01274. This unfortunately seems like a bug, we
        // should still have > 0 sizes.
        actual_extent.width = clamp(
            actual_extent.width,
            surface_capabilities.min_image_extent.width,
            surface_capabilities.max_image_extent.width,
        )
        .max(1);
        actual_extent.height = clamp(
            actual_extent.height,
            surface_capabilities.min_image_extent.height,
            surface_capabilities.max_image_extent.height,
        )
        .max(1);

        log::debug!("chose swapchain extents {:?}", actual_extent);
        actual_extent
    }

    fn choose_present_queue_family_index(
        surface: vk::SurfaceKHR,
        surface_loader: &Arc<khr::Surface>,
        physical_device: vk::PhysicalDevice,
        all_queue_families: &[vk::QueueFamilyProperties],
        graphics_queue_family_index: u32,
    ) -> VkResult<u32> {
        let graphics_queue_family_supports_present = unsafe {
            log::debug!("Use the graphics queue family to present");
            surface_loader.get_physical_device_surface_support(
                physical_device,
                graphics_queue_family_index,
                surface,
            )?
        };

        if graphics_queue_family_supports_present {
            // The graphics queue family will work
            Ok(graphics_queue_family_index)
        } else {
            // Try to find any queue family that can present
            for (queue_family_index, _) in all_queue_families.iter().enumerate() {
                let queue_family_index = queue_family_index as u32;

                log::debug!("Use dedicated present queue family");
                let supports_present = unsafe {
                    surface_loader.get_physical_device_surface_support(
                        physical_device,
                        graphics_queue_family_index,
                        surface,
                    )?
                };

                if supports_present {
                    // Present queue family found, return it
                    return Ok(queue_family_index);
                }
            }

            // Could not find any present queue family
            log::error!("Could not find suitable present queue family");
            Err(vk::Result::ERROR_UNKNOWN)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn create_swapchain(
        device_context: &RafxDeviceContextVulkan,
        surface: vk::SurfaceKHR,
        surface_capabilities: &vk::SurfaceCapabilitiesKHR,
        surface_format: vk::SurfaceFormatKHR,
        extents: vk::Extent2D,
        present_mode: vk::PresentModeKHR,
        swapchain_image_usage_flags: vk::ImageUsageFlags,
        old_swapchain: Option<vk::SwapchainKHR>,
        present_queue_family_index: u32,
    ) -> VkResult<CreateSwapchainResult> {
        log::trace!("VkSwapchain::create_swapchain");
        // "simply sticking to this minimum means that we may sometimes have to wait on the driver
        // to complete internal operations before we can acquire another image to render to.
        // Therefore it is recommended to request at least one more image than the minimum"
        let mut min_image_count = surface_capabilities.min_image_count + 1;

        // But if there is a limit, we must not exceed it
        if surface_capabilities.max_image_count > 0 {
            min_image_count = u32::min(min_image_count, surface_capabilities.max_image_count);
        }

        let swapchain_loader =
            khr::Swapchain::new(device_context.instance(), device_context.device());

        let mut swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(min_image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extents)
            .image_array_layers(1)
            .image_usage(swapchain_image_usage_flags)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        if let Some(old_swapchain) = old_swapchain {
            log::trace!("include old swapchain in swapchain_create_info");
            swapchain_create_info = swapchain_create_info.old_swapchain(old_swapchain);
        }

        // We must choose concurrent or exclusive image sharing mode. We only choose concurrent if
        // the queue families are not the same, which is uncommon. If we do choose concurrent, we
        // must provide this list of queue families.
        let queue_families = [
            device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            present_queue_family_index,
        ];

        let mut dedicated_present_queue = None;
        if device_context
            .queue_family_indices()
            .graphics_queue_family_index
            != present_queue_family_index
        {
            swapchain_create_info = swapchain_create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_families);

            dedicated_present_queue = Some(unsafe {
                device_context
                    .device()
                    .get_device_queue(present_queue_family_index, 0)
            });
        }

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        Ok(CreateSwapchainResult {
            swapchain_loader,
            swapchain,
            dedicated_present_queue,
        })
    }

    pub fn choose_sample_count(
        limits: &vk::PhysicalDeviceLimits,
        sample_count_priority: &[RafxSampleCount],
    ) -> RafxSampleCount {
        for &sample_count in sample_count_priority {
            let vk_sample_count: vk::SampleCountFlags = sample_count.into();
            if (vk_sample_count.as_raw()
                & limits.framebuffer_depth_sample_counts.as_raw()
                & limits.framebuffer_color_sample_counts.as_raw())
                != 0
            {
                log::trace!("Sample count {:?} is supported", sample_count);
                return sample_count;
            } else {
                log::trace!("Sample count {:?} is unsupported", sample_count);
            }
        }

        log::trace!(
            "None of the provided MSAA levels are supported defaulting to {:?}",
            RafxSampleCount::SampleCount1
        );
        RafxSampleCount::SampleCount1
    }
}

impl Drop for VkSwapchain {
    fn drop(&mut self) {
        log::trace!("destroying VkSwapchain");

        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }

        log::trace!("destroyed VkSwapchain");
    }
}
