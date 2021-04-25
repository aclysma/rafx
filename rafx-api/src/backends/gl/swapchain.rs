use crate::backends::gl::RafxTextureGl;
use crate::gl::{
    GlContext, RafxDeviceContextGl, RafxFenceGl, RafxRawImageGl, RafxSemaphoreGl, NONE_RENDERBUFFER,
};
use crate::{
    RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxSampleCount, RafxSwapchainDef,
    RafxSwapchainImage, RafxTexture, RafxTextureDef, RafxTextureDimensions,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

const SWAPCHAIN_IMAGE_COUNT: u32 = 3;
const SWAPCHAIN_FORMAT: RafxFormat = RafxFormat::R8G8B8A8_UNORM;

pub struct RafxSwapchainGl {
    device_context: RafxDeviceContextGl,
    surface_context: Arc<GlContext>,
    //layer: gl_rs::GlLayer,
    //drawable: TrustCell<Option<gl_rs::GlDrawable>>,
    swapchain_def: RafxSwapchainDef,
    format: RafxFormat,
    // Just fake this
    next_swapchain_image_index: u32,
    //swapchain_images: Vec<RafxTextureGl>,
    pub(crate) swapchain_image: RafxTextureGl,
}

impl RafxSwapchainGl {
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

    // pub fn gl_layer(&self) -> &gl_rs::GlLayerRef {
    //     self.layer.as_ref()
    // }

    // pub(crate) fn take_drawable(&self) -> Option<gl_rs::GlDrawable> {
    //     self.drawable.borrow_mut().take()
    // }

    pub fn new(
        device_context: &RafxDeviceContextGl,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainGl> {
        let surface_context = device_context
            .gl_context_manager()
            .create_surface_context(raw_window_handle)?;

        //TODO: set GL swap interval (vsync)

        let swapchain_image = Self::create_swapchain_image(device_context, swapchain_def)?;

        Ok(RafxSwapchainGl {
            device_context: device_context.clone(),
            surface_context,
            swapchain_def: swapchain_def.clone(),
            next_swapchain_image_index: 0,
            format: SWAPCHAIN_FORMAT,
            //swapchain_images,
            swapchain_image
        })
    }

    fn create_swapchain_image(device_context: &RafxDeviceContextGl, swapchain_def: &RafxSwapchainDef) -> RafxResult<RafxTextureGl> {
        RafxTextureGl::new(
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
        fence: &RafxFenceGl,
    ) -> RafxResult<RafxSwapchainImage> {
        fence.set_submitted(true);
        self.acquire_next_image()
    }

    pub fn acquire_next_image_semaphore(
        &mut self,
        semaphore: &RafxSemaphoreGl,
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
            texture: RafxTexture::Gl(self.swapchain_image.clone()),
            swapchain_image_index,
        })
    }
}
