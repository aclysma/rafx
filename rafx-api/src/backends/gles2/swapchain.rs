use crate::backends::gles2::RafxTextureGles2;
use crate::gles2::{
    GlContext, RafxDeviceContextGles2, RafxFenceGles2, RafxSemaphoreGles2,
};
use crate::{
    RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxSampleCount, RafxSwapchainDef,
    RafxSwapchainImage, RafxTexture, RafxTextureDef, RafxTextureDimensions,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

const SWAPCHAIN_IMAGE_COUNT: u32 = 3;
const SWAPCHAIN_FORMAT: RafxFormat = RafxFormat::R8G8B8A8_UNORM;

pub struct RafxSwapchainGles2 {
    device_context: RafxDeviceContextGles2,
    surface_context: Arc<GlContext>,
    swapchain_def: RafxSwapchainDef,
    format: RafxFormat,
    // Just fake this
    next_swapchain_image_index: u32,
    pub(crate) swapchain_image: RafxTextureGles2,
}

impl RafxSwapchainGles2 {
    pub fn swapchain_def(&self) -> &RafxSwapchainDef {
        &self.swapchain_def
    }

    pub fn image_count(&self) -> usize {
        SWAPCHAIN_IMAGE_COUNT as usize
    }

    pub fn format(&self) -> RafxFormat {
        self.format
    }

    pub fn surface_context(&self) -> &Arc<GlContext> {
        &self.surface_context
    }

    pub fn new(
        device_context: &RafxDeviceContextGles2,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainGles2> {
        let surface_context = device_context
            .gl_context_manager()
            .create_surface_context(raw_window_handle)?;

        //TODO: set GL swap interval (vsync)

        let swapchain_image = Self::create_swapchain_image(device_context, swapchain_def)?;

        Ok(RafxSwapchainGles2 {
            device_context: device_context.clone(),
            surface_context,
            swapchain_def: swapchain_def.clone(),
            next_swapchain_image_index: 0,
            format: SWAPCHAIN_FORMAT,
            swapchain_image
        })
    }

    fn create_swapchain_image(device_context: &RafxDeviceContextGles2, swapchain_def: &RafxSwapchainDef) -> RafxResult<RafxTextureGles2> {
        RafxTextureGles2::new(
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
            }
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
        fence: &RafxFenceGles2,
    ) -> RafxResult<RafxSwapchainImage> {
        fence.set_submitted(true);
        self.acquire_next_image()
    }

    pub fn acquire_next_image_semaphore(
        &mut self,
        semaphore: &RafxSemaphoreGles2,
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
            texture: RafxTexture::Gles2(self.swapchain_image.clone()),
            swapchain_image_index,
        })
    }
}
