use ash::extensions::khr;
use ash::prelude::VkResult;
use ash::vk;

use ash::version::{DeviceV1_0, InstanceV1_0};

use super::Window;
use crate::{MsaaLevel, PresentMode, VkDeviceContext};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

struct CreateSwapchainResult {
    swapchain_loader: khr::Swapchain,
    swapchain: vk::SwapchainKHR,
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
    pub device_context: VkDeviceContext,

    pub swapchain_info: SwapchainInfo,
    pub swapchain_loader: khr::Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,

    // One per MAX_FRAMES_IN_FLIGHT
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
}

impl VkSwapchain {
    pub fn new(
        device_context: &VkDeviceContext,
        window: &dyn Window,
        old_swapchain: Option<vk::SwapchainKHR>,
        present_mode_priority: &[PresentMode],
    ) -> VkResult<VkSwapchain> {
        let (available_formats, available_present_modes, surface_capabilities) =
            Self::query_swapchain_support(
                device_context.physical_device(),
                device_context.surface_loader(),
                device_context.surface(),
            )?;

        let surface_format = Self::choose_swapchain_format(&available_formats);
        info!("Surface format: {:?}", surface_format);

        let present_mode =
            Self::choose_present_mode(&available_present_modes, present_mode_priority);
        info!("Present mode: {:?}", present_mode);

        let extents = Self::choose_extents(&surface_capabilities, window);
        info!("Extents: {:?}", extents);

        let swapchain_image_usage_flags = vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let create_swapchain_result = Self::create_swapchain(
            device_context,
            &surface_capabilities,
            surface_format,
            extents,
            present_mode,
            swapchain_image_usage_flags,
            old_swapchain,
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

        let image_available_semaphores = Self::allocate_semaphores_per_frame(&device_context)?;
        let render_finished_semaphores = Self::allocate_semaphores_per_frame(&device_context)?;
        let in_flight_fences = Self::allocate_fences_per_frame(&device_context)?;

        Ok(VkSwapchain {
            device_context: device_context.clone(),
            swapchain_info,
            swapchain_loader: create_swapchain_result.swapchain_loader,
            swapchain: create_swapchain_result.swapchain,
            swapchain_images,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
        })
    }

    fn query_swapchain_support(
        physical_device: ash::vk::PhysicalDevice,
        surface_loader: &ash::extensions::khr::Surface,
        surface: ash::vk::SurfaceKHR,
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
        present_mode_priority: &[PresentMode],
    ) -> vk::PresentModeKHR {
        info!("Available present modes: {:?}", available_present_modes);
        info!("Preferred present modes: {:?}", present_mode_priority);

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
        window: &dyn Window,
    ) -> ash::vk::Extent2D {
        if surface_capabilities.current_extent.width != std::u32::MAX {
            debug!(
                "Swapchain extents chosen by surface capabilities ({} {})",
                surface_capabilities.current_extent.width,
                surface_capabilities.current_extent.height
            );
            surface_capabilities.current_extent
        } else {
            let physical_size = window.physical_size();

            debug!(
                "Swapchain extents chosen by inner window size ({} {})",
                physical_size.width, physical_size.height
            );

            let mut actual_extent = ash::vk::Extent2D::builder()
                .width(physical_size.width)
                .height(physical_size.height)
                .build();

            // Copied from num-traits under MIT/Apache-2.0 dual license. It doesn't make much sense
            // to pull in a whole crate just for this utility function
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

            actual_extent.width = clamp(
                actual_extent.width,
                surface_capabilities.min_image_extent.width,
                surface_capabilities.max_image_extent.width,
            );
            actual_extent.height = clamp(
                actual_extent.height,
                surface_capabilities.min_image_extent.height,
                surface_capabilities.max_image_extent.height,
            );

            actual_extent
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn create_swapchain(
        device_context: &VkDeviceContext,
        surface_capabilities: &vk::SurfaceCapabilitiesKHR,
        surface_format: vk::SurfaceFormatKHR,
        extents: vk::Extent2D,
        present_mode: vk::PresentModeKHR,
        swapchain_image_usage_flags: vk::ImageUsageFlags,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> VkResult<CreateSwapchainResult> {
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
            .surface(device_context.surface())
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
            swapchain_create_info = swapchain_create_info.old_swapchain(old_swapchain);
        }

        // We must choose concurrent or exclusive image sharing mode. We only choose concurrent if
        // the queue families are not the same, which is uncommon. If we do choose concurrent, we
        // must provide this list of queue families.
        let queue_families = [
            device_context
                .queue_family_indices()
                .graphics_queue_family_index,
            device_context
                .queue_family_indices()
                .present_queue_family_index,
        ];

        if device_context
            .queue_family_indices()
            .graphics_queue_family_index
            != device_context
                .queue_family_indices()
                .present_queue_family_index
        {
            swapchain_create_info = swapchain_create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_families);
        }

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        Ok(CreateSwapchainResult {
            swapchain_loader,
            swapchain,
        })
    }

    pub fn choose_msaa_level(
        limits: &vk::PhysicalDeviceLimits,
        msaa_level_priority: &[MsaaLevel],
    ) -> MsaaLevel {
        for msaa_level in msaa_level_priority {
            let sample_count: vk::SampleCountFlags = msaa_level.clone().into();
            if (sample_count.as_raw()
                & limits.framebuffer_depth_sample_counts.as_raw()
                & limits.framebuffer_color_sample_counts.as_raw())
                != 0
            {
                log::trace!("MSAA level {:?} is supported", msaa_level);
                return *msaa_level;
            } else {
                log::trace!("MSAA level {:?} is unsupported", msaa_level);
            }
        }

        log::trace!(
            "None of the provided MSAA levels are supported defaulting to {:?}",
            MsaaLevel::Sample1
        );
        MsaaLevel::Sample1
    }

    pub fn find_supported_format(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        candidates: &[vk::Format],
        image_tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Option<vk::Format> {
        for candidate in candidates {
            let props = unsafe {
                instance.get_physical_device_format_properties(physical_device, *candidate)
            };

            let is_supported = match image_tiling {
                vk::ImageTiling::LINEAR => (props.linear_tiling_features & features) == features,
                vk::ImageTiling::OPTIMAL => (props.optimal_tiling_features & features) == features,
                _ => unimplemented!(),
            };

            if is_supported {
                return Some(*candidate);
            }
        }

        None
    }

    pub fn choose_supported_format(
        device_context: &VkDeviceContext,
        candidates: &[vk::Format],
        format_features: vk::FormatFeatureFlags,
    ) -> vk::Format {
        let format = Self::find_supported_format(
            device_context.instance(),
            device_context.physical_device(),
            candidates,
            vk::ImageTiling::OPTIMAL,
            format_features,
        )
        .unwrap();

        format
    }

    fn allocate_semaphores_per_frame(
        device_context: &VkDeviceContext
    ) -> VkResult<Vec<vk::Semaphore>> {
        let mut semaphores = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let semaphore_create_info = vk::SemaphoreCreateInfo::builder();
            let semaphore = unsafe {
                device_context
                    .device()
                    .create_semaphore(&semaphore_create_info, None)?
            };
            semaphores.push(semaphore);
        }

        Ok(semaphores)
    }

    fn allocate_fences_per_frame(device_context: &VkDeviceContext) -> VkResult<Vec<vk::Fence>> {
        let mut fences = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let fence_create_info =
                vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

            let fence = unsafe {
                device_context
                    .device()
                    .create_fence(&fence_create_info, None)?
            };
            fences.push(fence);
        }

        Ok(fences)
    }
}

impl Drop for VkSwapchain {
    fn drop(&mut self) {
        trace!("destroying VkSwapchain");

        unsafe {
            let device = self.device_context.device();
            for &semaphore in self.image_available_semaphores.iter() {
                device.destroy_semaphore(semaphore, None);
            }

            for &semaphore in self.render_finished_semaphores.iter() {
                device.destroy_semaphore(semaphore, None);
            }

            for &fence in self.in_flight_fences.iter() {
                device.destroy_fence(fence, None);
            }

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }

        trace!("destroyed VkSwapchain");
    }
}
