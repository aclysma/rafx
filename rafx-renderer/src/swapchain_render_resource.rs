use rafx_api::{RafxDeviceContext, RafxFormat, RafxResourceType, RafxResult};
use rafx_framework::graph::SwapchainSurfaceInfo;

pub struct SwapchainRenderResourceInner {
    // The images presented by the swapchain
    //TODO: We don't properly support multiple swapchains right now. This would ideally be a map
    // of window/surface to info for the swapchain
    //pub swapchain_images: Vec<ResourceArc<ImageViewResource>>,
    pub swapchain_surface_info: SwapchainSurfaceInfo,

    pub default_color_format_hdr: RafxFormat,
    pub default_color_format_sdr: RafxFormat,
    pub default_depth_format: RafxFormat,
}

// Contents are none if a swapchain does not exist. We allow this state so that we can insert this
// resource into the render resources map on init while we still have mut access to it, and not
// require adding/removing it when we create/destroy the swapchain
#[derive(Default)]
pub struct SwapchainRenderResource(Option<SwapchainRenderResourceInner>);

impl SwapchainRenderResource {
    pub fn set_swapchain(
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

        self.0 = Some(SwapchainRenderResourceInner {
            swapchain_surface_info,
            default_color_format_hdr,
            default_color_format_sdr,
            default_depth_format,
        });

        Ok(())
    }

    pub fn clear_swapchain(&mut self) {
        self.0 = None;
    }

    pub fn get(&self) -> Option<&SwapchainRenderResourceInner> {
        self.0.as_ref()
    }
}
