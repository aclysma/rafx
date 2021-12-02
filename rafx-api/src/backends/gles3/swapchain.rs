use crate::backends::gles3::RafxTextureGles3;
use crate::gles3::{GlContext, RafxDeviceContextGles3, RafxFenceGles3, RafxSemaphoreGles3};
use crate::{
    RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxSampleCount,
    RafxSwapchainColorSpace, RafxSwapchainDef, RafxSwapchainImage, RafxTexture, RafxTextureDef,
    RafxTextureDimensions,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

const SWAPCHAIN_IMAGE_COUNT: u32 = 3;
const SWAPCHAIN_FORMAT: RafxFormat = RafxFormat::R8G8B8A8_SRGB;

pub struct RafxSwapchainGles3 {
    device_context: RafxDeviceContextGles3,
    surface_context: Arc<GlContext>,
    swapchain_def: RafxSwapchainDef,
    format: RafxFormat,
    // Just fake this
    next_swapchain_image_index: u32,
    pub(crate) swapchain_image: RafxTextureGles3,
}

impl RafxSwapchainGles3 {
    pub fn swapchain_def(&self) -> &RafxSwapchainDef {
        &self.swapchain_def
    }

    pub fn image_count(&self) -> usize {
        SWAPCHAIN_IMAGE_COUNT as usize
    }

    pub fn format(&self) -> RafxFormat {
        self.format
    }

    pub fn color_space(&self) -> RafxSwapchainColorSpace {
        self.swapchain_def.color_space
    }

    pub fn surface_context(&self) -> &Arc<GlContext> {
        &self.surface_context
    }

    pub fn new(
        device_context: &RafxDeviceContextGles3,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainGles3> {
        let surface_context = device_context
            .gl_context_manager()
            .create_surface_context(raw_window_handle)?;

        if swapchain_def.color_space != RafxSwapchainColorSpace::Srgb {
            unimplemented!("GLES3 backend only supports sRGB Non-Linear color space");
        }

        //TODO: GL swap interval is not being set (vsync). Doesn't seem to be a good cross-platform way to do
        // this. And some platforms don't respect it even if a configuration method is present.

        let swapchain_image = Self::create_swapchain_image(device_context, swapchain_def)?;

        Ok(RafxSwapchainGles3 {
            device_context: device_context.clone(),
            surface_context,
            swapchain_def: swapchain_def.clone(),
            next_swapchain_image_index: 0,
            format: SWAPCHAIN_FORMAT,
            swapchain_image,
        })
    }

    fn create_swapchain_image(
        device_context: &RafxDeviceContextGles3,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxTextureGles3> {
        RafxTextureGles3::new(
            device_context,
            &RafxTextureDef {
                extents: RafxExtents3D {
                    width: swapchain_def.width,
                    height: swapchain_def.height,
                    depth: 1,
                },
                array_length: 1,
                mip_count: 1,
                format: SWAPCHAIN_FORMAT,
                resource_type: RafxResourceType::TEXTURE | RafxResourceType::RENDER_TARGET_COLOR,
                sample_count: RafxSampleCount::SampleCount1,
                dimensions: RafxTextureDimensions::Dim2D,
            },
        )
    }

    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        self.swapchain_def = swapchain_def.clone();
        self.swapchain_image = Self::create_swapchain_image(&self.device_context, swapchain_def)?;
        Ok(())
    }

    pub fn acquire_next_image_fence(
        &mut self,
        fence: &RafxFenceGles3,
    ) -> RafxResult<RafxSwapchainImage> {
        fence.set_submitted(true);
        self.acquire_next_image()
    }

    pub fn acquire_next_image_semaphore(
        &mut self,
        semaphore: &RafxSemaphoreGles3,
    ) -> RafxResult<RafxSwapchainImage> {
        semaphore.set_signal_available(true);
        self.acquire_next_image()
    }

    pub fn acquire_next_image(&mut self) -> RafxResult<RafxSwapchainImage> {
        let swapchain_image_index = self.next_swapchain_image_index;
        self.next_swapchain_image_index += 1;
        if self.next_swapchain_image_index >= SWAPCHAIN_IMAGE_COUNT {
            self.next_swapchain_image_index = 0;
        }

        Ok(RafxSwapchainImage {
            texture: RafxTexture::Gles3(self.swapchain_image.clone()),
            swapchain_image_index,
        })
    }
}
