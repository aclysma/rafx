use rafx_api::{RafxDeviceContext, RafxFormat, RafxResourceType, RafxResult};
use rafx_framework::graph::SwapchainSurfaceInfo;

pub struct SwapchainRenderResourceSurfaceInfo {
    // The images presented by the swapchain
    //TODO: We don't properly support multiple swapchains right now. This would ideally be a map
    // of window/surface to info for the swapchain
    //pub swapchain_images: Vec<ResourceArc<ImageViewResource>>,
    pub swapchain_surface_info: SwapchainSurfaceInfo,

    pub default_color_format_hdr: RafxFormat,
    pub default_color_format_sdr: RafxFormat,
    pub default_depth_format: RafxFormat,
}

#[derive(Default)]
pub struct SwapchainRenderResource {
    // Contents are none if a swapchain does not exist. We allow this state so that we can insert this
    // resource into the render resources map on init while we still have mut access to it, and not
    // require adding/removing it when we create/destroy the swapchain
    surface_info: Option<SwapchainRenderResourceSurfaceInfo>,
    pub max_color_component_value: f32,
}

impl SwapchainRenderResource {
    pub fn set_swapchain_info(
        &mut self,
        device_context: &RafxDeviceContext,
        swapchain_surface_info: SwapchainSurfaceInfo,
    ) -> RafxResult<()> {
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

        self.surface_info = Some(SwapchainRenderResourceSurfaceInfo {
            swapchain_surface_info,
            default_color_format_hdr,
            default_color_format_sdr,
            default_depth_format,
        });

        Ok(())
    }

    pub fn clear_swapchain_info(&mut self) {
        self.surface_info = None;
    }

    /// Set the max drawable color component value. On SDR displays, this should be 1. On HDR
    /// displays, this should be > 1.0. This value should be informed by the OS. On apple devices,
    /// even non-HDR displays may be > 1.0 if there is enough additional "brightness headroom" to
    /// produce an HDR-like effect.
    pub fn set_max_color_component_value(
        &mut self,
        max_value: f32,
    ) {
        self.max_color_component_value = max_value;
    }

    pub fn surface_info(&self) -> Option<&SwapchainRenderResourceSurfaceInfo> {
        self.surface_info.as_ref()
    }
}
