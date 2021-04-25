use crate::backends::gl::RafxTextureGl;
use crate::gl::{RafxDeviceContextGl, RafxFenceGl, RafxRawImageGl, RafxSemaphoreGl, GlContext, NONE_RENDERBUFFER};
use crate::{
    RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxSampleCount, RafxSwapchainDef,
    RafxSwapchainImage, RafxTexture, RafxTextureDef, RafxTextureDimensions,
};
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

const SWAPCHAIN_IMAGE_COUNT: u32 = 3;

pub struct RafxSwapchainGl {
    _device_context: RafxDeviceContextGl,
    surface_context: Arc<GlContext>,
    //layer: gl_rs::GlLayer,
    //drawable: TrustCell<Option<gl_rs::GlDrawable>>,
    swapchain_def: RafxSwapchainDef,
    format: RafxFormat,
    // Just fake this
    next_swapchain_image_index: u32,
    swapchain_images: Vec<RafxTextureGl>,
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
        let format = RafxFormat::R8G8B8A8_UNORM;

        let resource_type = RafxResourceType::TEXTURE | RafxResourceType::RENDER_TARGET_COLOR;

        let surface_context = device_context.gl_context_manager().create_surface_context(raw_window_handle)?;

        //TODO: set GL swap interval (vsync)

        let mut swapchain_images = Vec::with_capacity(SWAPCHAIN_IMAGE_COUNT as usize);
        for _ in 0..SWAPCHAIN_IMAGE_COUNT {
            swapchain_images.push(RafxTextureGl::from_existing(device_context, Some(RafxRawImageGl::Renderbuffer(NONE_RENDERBUFFER)), &RafxTextureDef {
                extents: RafxExtents3D {
                    width: swapchain_def.width,
                    height: swapchain_def.height,
                    depth: 1
                },
                array_length: 1,
                mip_count: 1,
                format,
                resource_type,
                sample_count: RafxSampleCount::SampleCount1,
                dimensions: RafxTextureDimensions::Dim2D,
            })?);
        }

        Ok(RafxSwapchainGl {
            _device_context: device_context.clone(),
            surface_context,
            swapchain_def: swapchain_def.clone(),
            next_swapchain_image_index: 0,
            format,
            swapchain_images,
        })
    }

    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        self.swapchain_def = swapchain_def.clone();
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
            texture: RafxTexture::Gl(self.swapchain_images[swapchain_image_index as usize].clone()),
            swapchain_image_index,
        })
    }
}
