use crate::backends::gl::RafxTextureGl;
use crate::gl::{RafxDeviceContextGl, RafxFenceGl, RafxRawImageGl, RafxSemaphoreGl, RafxTextureGlInner, GlContext};
use crate::{
    RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxSampleCount, RafxSwapchainDef,
    RafxSwapchainImage, RafxTexture, RafxTextureDef, RafxTextureDimensions,
};
use rafx_base::trust_cell::TrustCell;
use raw_window_handle::HasRawWindowHandle;

const SWAPCHAIN_IMAGE_COUNT: u32 = 3;

pub struct RafxSwapchainGl {
    device_context: RafxDeviceContextGl,
    //layer: gl_rs::GlLayer,
    //drawable: TrustCell<Option<gl_rs::GlDrawable>>,
    swapchain_def: RafxSwapchainDef,
    format: RafxFormat,
    // Just fake this
    next_swapchain_image_index: u32,
    swapchain_images: Vec<RafxTextureGl>,
}

// for gl_rs::CAGlDrawable
unsafe impl Send for RafxSwapchainGl {}
unsafe impl Sync for RafxSwapchainGl {}

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
        let format = RafxFormat::B8G8R8A8_SRGB;

        let mut resource_type = RafxResourceType::TEXTURE | RafxResourceType::RENDER_TARGET_COLOR;

        let context = device_context.gl_context_manager().create_surface_context(raw_window_handle);

        // add surface
        // set GL swap interval (vsync)

        let mut swapchain_images = Vec::with_capacity(SWAPCHAIN_IMAGE_COUNT as usize);
        for _ in 0..SWAPCHAIN_IMAGE_COUNT {
            swapchain_images.push(RafxTextureGl::from_existing(device_context, Some(RafxRawImageGl::RenderTarget), &RafxTextureDef {
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
            device_context: device_context.clone(),
            //layer,
            //drawable: Default::default(),
            swapchain_def: swapchain_def.clone(),
            next_swapchain_image_index: 0,
            format,
            swapchain_images,
        })


        //unimplemented!();
        // let layer = match raw_window_handle.raw_window_handle() {
        //     #[cfg(target_os = "macos")]
        //     raw_window_handle::RawWindowHandle::MacOS(handle) => unsafe {
        //         raw_window_gl::macos::gl_layer_from_handle(handle)
        //     },
        //     #[cfg(target_os = "ios")]
        //     raw_window_handle::RawWindowHandle::IOS(handle) => unsafe {
        //         raw_window_gl::ios::gl_layer_from_handle(handle)
        //     },
        //     _ => return Err("Cannot create RafxSurfaceGl on this operating system".into()),
        // };
        //
        // let layer = match layer {
        //     raw_window_gl::Layer::Allocated(x) => Some(x),
        //     raw_window_gl::Layer::Existing(x) => Some(x),
        //     raw_window_gl::Layer::None => None,
        // }
        // .unwrap();
        //
        // let layer = unsafe { std::mem::transmute::<_, &gl_rs::GlLayerRef>(layer).to_owned() };
        //
        // layer.set_device(device_context.device());
        // //TODO: Don't hardcode pixel format
        // // https://developer.apple.com/documentation/quartzcore/cagllayer/1478155-pixelformat
        // layer.set_pixel_format(gl_rs::MTLPixelFormat::BGRA8Unorm_sRGB);
        // layer.set_presents_with_transaction(false);
        // layer.set_display_sync_enabled(swapchain_def.enable_vsync);
        //
        // //TODO: disable timeout on acquire drawable?
        // layer.set_drawable_size(gl_rs::CGSize::new(
        //     swapchain_def.width as f64,
        //     swapchain_def.height as f64,
        // ));
        //
        // let swapchain_def = swapchain_def.clone();
        //
        // Ok(RafxSwapchainGl {
        //     device_context: device_context.clone(),
        //     layer,
        //     drawable: Default::default(),
        //     swapchain_def,
        //     next_swapchain_image_index: 0,
        //     format: RafxFormat::B8G8R8A8_SRGB,
        // })
    }

    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        unimplemented!();
        // self.layer.set_drawable_size(gl_rs::CGSize::new(
        //     swapchain_def.width as f64,
        //     swapchain_def.height as f64,
        // ));
        // //TODO: Add to gl crate, following presents_with_transaction as an example
        // //self.layer.set_display_sync_enabled(swapchain_def.enable_vsync);
        //
        // self.swapchain_def = swapchain_def.clone();
        Ok(())
    }

    pub fn acquire_next_image_fence(
        &mut self,
        _fence: &RafxFenceGl,
    ) -> RafxResult<RafxSwapchainImage> {
        unimplemented!();
        self.acquire_next_image()
    }

    pub fn acquire_next_image_semaphore(
        &mut self,
        _semaphore: &RafxSemaphoreGl,
    ) -> RafxResult<RafxSwapchainImage> {
        unimplemented!();
        self.acquire_next_image()
    }

    pub fn acquire_next_image(&mut self) -> RafxResult<RafxSwapchainImage> {
        unimplemented!();
        // objc::rc::autoreleasepool(|| {
        //     let drawable = self
        //         .layer
        //         .next_drawable()
        //         .ok_or("Timed out while trying to acquire drawable".to_string())?;
        //
        //     let mut old_drawable = self.drawable.borrow_mut();
        //     assert!(old_drawable.is_none());
        //     *old_drawable = Some(drawable.to_owned());
        //
        //     let raw_image = RafxRawImageGl::Ref(drawable.texture().to_owned());
        //
        //     // This ends up being cheap because it doesn't allocate anything. We could cache it but it doesn't
        //     // seem worthwhile
        //     let image = RafxTextureGl::from_existing(
        //         &self.device_context,
        //         Some(raw_image),
        //         &RafxTextureDef {
        //             extents: RafxExtents3D {
        //                 width: self.swapchain_def.width,
        //                 height: self.swapchain_def.height,
        //                 depth: 1,
        //             },
        //             array_length: 1,
        //             mip_count: 1,
        //             format: self.format,
        //             resource_type: RafxResourceType::UNDEFINED,
        //             sample_count: RafxSampleCount::SampleCount1,
        //             dimensions: RafxTextureDimensions::Dim2D,
        //         },
        //     )?;
        //
        //     let swapchain_image_index = self.next_swapchain_image_index;
        //     self.next_swapchain_image_index += 1;
        //     if self.next_swapchain_image_index >= SWAPCHAIN_IMAGE_COUNT {
        //         self.next_swapchain_image_index = 0;
        //     }
        //
        //     Ok(RafxSwapchainImage {
        //         texture: RafxTexture::Gl(image),
        //         swapchain_image_index,
        //     })
        // })
    }
}
