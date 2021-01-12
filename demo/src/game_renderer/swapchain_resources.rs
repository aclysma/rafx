use crate::game_renderer::GameRendererInner;
use ash::vk;
use rafx::api::{RafxDeviceContext, RafxFormat, RafxResult, RafxSwapchain};
use rafx::resources::graph::SwapchainSurfaceInfo;
use rafx::resources::ResourceManager;

pub struct SwapchainResources {
    // The images presented by the swapchain
    //TODO: We don't properly support multiple swapchains right now. This would ideally be a map
    // of window/surface to info for the swapchain
    //pub swapchain_images: Vec<ResourceArc<ImageViewResource>>,
    pub swapchain_surface_info: SwapchainSurfaceInfo,

    pub default_color_format_hdr: RafxFormat,
    pub default_color_format_sdr: RafxFormat,
    pub default_depth_format: RafxFormat,
}

impl SwapchainResources {
    pub fn new(
        device_context: &RafxDeviceContext,
        _swapchain: &RafxSwapchain,
        _game_renderer: &mut GameRendererInner,
        _resource_manager: &mut ResourceManager,
        swapchain_surface_info: SwapchainSurfaceInfo,
    ) -> RafxResult<SwapchainResources> {
        log::debug!("creating swapchain resources");

        //
        // Determine default color formats
        //
        let default_color_format_hdr = device_context
            .vk_device_context()
            .unwrap()
            .find_supported_format(
                &rafx::api::vulkan::DEFAULT_COLOR_FORMATS_HDR,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::COLOR_ATTACHMENT,
            )
            .ok_or_else(|| "Could not find a supported HDR color format")?;

        let default_color_format_sdr = swapchain_surface_info.format;

        let default_depth_format = device_context
            .vk_device_context()
            .unwrap()
            .find_supported_format(
                &rafx::api::vulkan::DEFAULT_DEPTH_FORMATS,
                vk::ImageTiling::OPTIMAL,
                vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
            )
            .ok_or_else(|| "Could not find a supported depth format")?;

        log::debug!("game renderer swapchain_created finished");

        Ok(SwapchainResources {
            //swapchain_images,
            swapchain_surface_info,
            default_color_format_hdr: default_color_format_hdr.into(),
            default_color_format_sdr: default_color_format_sdr.into(),
            default_depth_format: default_depth_format.into(),
        })
    }
}
