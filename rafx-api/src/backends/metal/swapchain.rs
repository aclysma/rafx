use super::extra_ffi;
use crate::backends::metal::RafxTextureMetal;
use crate::metal::{RafxDeviceContextMetal, RafxFenceMetal, RafxRawImageMetal, RafxSemaphoreMetal};
use crate::{
    RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxSampleCount,
    RafxSwapchainColorSpace, RafxSwapchainDef, RafxSwapchainImage, RafxTexture, RafxTextureDef,
    RafxTextureDimensions,
};
use core_graphics_types::geometry::CGSize;
use rafx_base::trust_cell::TrustCell;
use raw_window_handle::HasRawWindowHandle;

const SWAPCHAIN_IMAGE_COUNT: u32 = 3;

#[derive(Clone)]
pub struct RafxSwapchainMetalEdrInfo {
    pub max_edr_color_component_value: f32,
    pub max_potential_edr_color_component_value: f32,
    pub max_reference_edr_color_component_value: f32,
}

impl RafxSwapchainMetalEdrInfo {
    fn new(window: &extra_ffi::NSWindowWrapper) -> Self {
        RafxSwapchainMetalEdrInfo {
            max_edr_color_component_value: window.max_edr_color_component_value(),
            max_potential_edr_color_component_value: window
                .max_potential_edr_color_component_value(),
            max_reference_edr_color_component_value: window
                .max_reference_edr_color_component_value(),
        }
    }
}

pub struct RafxSwapchainMetal {
    device_context: RafxDeviceContextMetal,
    window: extra_ffi::NSWindowWrapper,
    layer: metal_rs::MetalLayer,
    drawable: TrustCell<Option<metal_rs::MetalDrawable>>,
    swapchain_def: RafxSwapchainDef,
    format: RafxFormat,
    color_space: RafxSwapchainColorSpace,
    // Just fake this
    next_swapchain_image_index: u32,
    edr_info: RafxSwapchainMetalEdrInfo,
}

// for metal_rs::CAMetalDrawable
unsafe impl Send for RafxSwapchainMetal {}
unsafe impl Sync for RafxSwapchainMetal {}

impl RafxSwapchainMetal {
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
        self.color_space
    }

    pub fn metal_layer(&self) -> &metal_rs::MetalLayerRef {
        self.layer.as_ref()
    }

    pub(crate) fn take_drawable(&self) -> Option<metal_rs::MetalDrawable> {
        self.drawable.borrow_mut().take()
    }

    //
    // Some extra metal-specific HDR-related values
    //
    pub fn edr_info(&self) -> &RafxSwapchainMetalEdrInfo {
        &self.edr_info
    }

    pub fn new(
        device_context: &RafxDeviceContextMetal,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainMetal> {
        let (window, _view, layer) = match raw_window_handle.raw_window_handle() {
            #[cfg(target_os = "macos")]
            raw_window_handle::RawWindowHandle::MacOS(handle) => unsafe {
                (
                    handle.ns_window,
                    handle.ns_view,
                    raw_window_metal::macos::metal_layer_from_handle(handle),
                )
            },
            #[cfg(target_os = "ios")]
            raw_window_handle::RawWindowHandle::IOS(handle) => unsafe {
                (
                    handle.ns_window,
                    handle.ns_view,
                    raw_window_metal::ios::metal_layer_from_handle(handle),
                )
            },
            _ => return Err("Cannot create RafxSurfaceMetal on this operating system".into()),
        };

        let layer = match layer {
            raw_window_metal::Layer::Allocated(x) => Some(x),
            raw_window_metal::Layer::Existing(x) => Some(x),
            raw_window_metal::Layer::None => None,
        }
        .unwrap();

        let layer = unsafe { std::mem::transmute::<_, &metal_rs::MetalLayerRef>(layer).to_owned() };
        let window = extra_ffi::NSWindowWrapper::new(window as *mut objc::runtime::Object);

        // Metal backend support all color spaces, so we can just use the first one in the list
        let preferred_color_space = swapchain_def.color_space_priority[0];

        let (pixel_format, swapchain_format, is_extended) = match preferred_color_space {
            RafxSwapchainColorSpace::Srgb => (
                metal_rs::MTLPixelFormat::BGRA8Unorm_sRGB,
                RafxFormat::B8G8R8A8_SRGB,
                false,
            ),
            RafxSwapchainColorSpace::SrgbExtended | RafxSwapchainColorSpace::DisplayP3Extended => (
                metal_rs::MTLPixelFormat::RGBA16Float,
                RafxFormat::R16G16B16A16_SFLOAT,
                true,
            ),
        };

        // Metal supports all color spaces
        let preferred_color_space = swapchain_def.color_space_priority[0];
        let color_space = core_graphics::color_space::CGColorSpace::create_with_name(
            preferred_color_space.into(),
        )
        .unwrap();

        layer.set_device(device_context.device());
        //TODO: Don't hardcode pixel format
        // https://developer.apple.com/documentation/quartzcore/cametallayer/1478155-pixelformat
        layer.set_pixel_format(pixel_format);
        layer.set_presents_with_transaction(false);
        layer.set_display_sync_enabled(swapchain_def.enable_vsync);
        layer.set_wants_extended_dynamic_range_content(is_extended);
        extra_ffi::set_colorspace(&layer, &color_space);

        //TODO: disable timeout on acquire drawable?
        layer.set_drawable_size(CGSize::new(
            swapchain_def.width as f64,
            swapchain_def.height as f64,
        ));

        let swapchain_def = swapchain_def.clone();
        let edr_info = RafxSwapchainMetalEdrInfo::new(&window);

        Ok(RafxSwapchainMetal {
            device_context: device_context.clone(),
            window,
            layer,
            drawable: Default::default(),
            swapchain_def,
            next_swapchain_image_index: 0,
            format: swapchain_format,
            color_space: preferred_color_space,
            edr_info,
        })
    }

    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        self.layer.set_drawable_size(CGSize::new(
            swapchain_def.width as f64,
            swapchain_def.height as f64,
        ));
        //TODO: Add to metal crate, following presents_with_transaction as an example
        //self.layer.set_display_sync_enabled(swapchain_def.enable_vsync);

        self.swapchain_def = swapchain_def.clone();
        Ok(())
    }

    pub fn acquire_next_image_fence(
        &mut self,
        _fence: &RafxFenceMetal,
    ) -> RafxResult<RafxSwapchainImage> {
        self.acquire_next_image()
    }

    pub fn acquire_next_image_semaphore(
        &mut self,
        _semaphore: &RafxSemaphoreMetal,
    ) -> RafxResult<RafxSwapchainImage> {
        self.acquire_next_image()
    }

    pub fn acquire_next_image(&mut self) -> RafxResult<RafxSwapchainImage> {
        objc::rc::autoreleasepool(|| {
            //TODO: next_drawable is always vsync-locked and is likely a higher-latency way to do things
            // It would be possible to submit render commands to an off-screen render target and then present
            // later.
            let drawable = self
                .layer
                .next_drawable()
                .ok_or("Timed out while trying to acquire drawable".to_string())?;

            let mut old_drawable = self.drawable.borrow_mut();
            assert!(old_drawable.is_none());
            *old_drawable = Some(drawable.to_owned());

            let raw_image = RafxRawImageMetal::Ref(drawable.texture().to_owned());

            let resource_type = RafxResourceType::TEXTURE | RafxResourceType::RENDER_TARGET_COLOR;

            // This ends up being cheap because it doesn't allocate anything. We could cache it but it doesn't
            // seem worthwhile
            let image = RafxTextureMetal::from_existing(
                &self.device_context,
                Some(raw_image),
                &RafxTextureDef {
                    extents: RafxExtents3D {
                        width: self.swapchain_def.width,
                        height: self.swapchain_def.height,
                        depth: 1,
                    },
                    array_length: 1,
                    mip_count: 1,
                    format: self.format,
                    resource_type,
                    sample_count: RafxSampleCount::SampleCount1,
                    dimensions: RafxTextureDimensions::Dim2D,
                },
            )?;

            let swapchain_image_index = self.next_swapchain_image_index;
            self.next_swapchain_image_index += 1;
            if self.next_swapchain_image_index >= SWAPCHAIN_IMAGE_COUNT {
                self.next_swapchain_image_index = 0;
            }

            self.edr_info = RafxSwapchainMetalEdrInfo::new(&self.window);

            Ok(RafxSwapchainImage {
                texture: RafxTexture::Metal(image),
                swapchain_image_index,
            })
        })
    }
}
