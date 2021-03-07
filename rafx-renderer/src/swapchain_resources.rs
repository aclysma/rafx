use rafx_api::{RafxDeviceContext, RafxFormat, RafxResourceType, RafxResult};
use rafx_framework::graph::SwapchainSurfaceInfo;

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
        swapchain_surface_info: SwapchainSurfaceInfo,
    ) -> RafxResult<SwapchainResources> {
        log::debug!("creating swapchain resources");

        // Use swapchain format for SDR color
        let default_color_format_sdr = swapchain_surface_info.format;

        let default_color_format_hdr = device_context
            .find_supported_format(
                &rafx_api::recommended_formats::COLOR_FORMATS_HDR,
                RafxResourceType::RENDER_TARGET_COLOR,
            )
            .ok_or_else(|| "Could not find a supported hdr color format")?;

        let default_depth_format = device_context
            .find_supported_format(
                &rafx_api::recommended_formats::DEPTH_FORMATS,
                RafxResourceType::RENDER_TARGET_DEPTH_STENCIL,
            )
            .ok_or_else(|| "Could not find a supported depth format")?;

        Ok(SwapchainResources {
            swapchain_surface_info,
            default_color_format_hdr,
            default_color_format_sdr,
            default_depth_format,
        })
    }
}
