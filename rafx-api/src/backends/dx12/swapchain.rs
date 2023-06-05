use crate::dx12::{
    RafxDeviceContextDx12, RafxFenceDx12, RafxQueueDx12, RafxRawImageDx12, RafxSemaphoreDx12,
    RafxTextureDx12,
};
use crate::{
    RafxExtents3D, RafxFormat, RafxResourceType, RafxResult, RafxSampleCount,
    RafxSwapchainColorSpace, RafxSwapchainDef, RafxSwapchainImage, RafxTexture, RafxTextureDef,
    RafxTextureDimensions,
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle, RawWindowHandle};
use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::IDXGISwapChain3;

use super::dxgi;

const SWAPCHAIN_IMAGE_COUNT: u32 = 3;

pub struct RafxSwapchainDx12 {
    device_context: RafxDeviceContextDx12,
    swapchain_def: RafxSwapchainDef,
    format: RafxFormat,
    color_space: RafxSwapchainColorSpace,
    swapchain_images: Vec<RafxSwapchainImage>,
    swapchain: dxgi::IDXGISwapChain3,
    swapchain_flags: dxgi::DXGI_SWAP_CHAIN_FLAG,
    present_queue: RafxQueueDx12,
}

// for dxgi::IDXGISwapChain3
unsafe impl Send for RafxSwapchainDx12 {}
unsafe impl Sync for RafxSwapchainDx12 {}

impl RafxSwapchainDx12 {
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

    pub fn dx12_swapchain(&self) -> &dxgi::IDXGISwapChain3 {
        &self.swapchain
    }

    pub fn swapchain_flags(&self) -> dxgi::DXGI_SWAP_CHAIN_FLAG {
        self.swapchain_flags
    }

    pub fn new(
        device_context: &RafxDeviceContextDx12,
        _raw_display_handle: &dyn HasRawDisplayHandle,
        raw_window_handle: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
        present_queue: &RafxQueueDx12,
    ) -> RafxResult<RafxSwapchainDx12> {
        //TODO: Select format
        let preferred_color_space = RafxSwapchainColorSpace::Srgb;
        //let preferred_color_space = swapchain_def.color_space_priority[0];
        // let (pixel_format, swapchain_format, is_extended) = match preferred_color_space {
        //     RafxSwapchainColorSpace::Srgb => (
        //         metal_rs::MTLPixelFormat::BGRA8Unorm_sRGB,
        //         RafxFormat::B8G8R8A8_SRGB,
        //         false,
        //     ),
        //     RafxSwapchainColorSpace::SrgbExtended | RafxSwapchainColorSpace::DisplayP3Extended => (
        //         metal_rs::MTLPixelFormat::RGBA16Float,
        //         RafxFormat::R16G16B16A16_SFLOAT,
        //         true,
        //     ),
        // };

        // Only DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R10G10B10A2_UNORM allowed?
        let pixel_format = dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
        let swapchain_format = RafxFormat::B8G8R8A8_SRGB;
        //let is_extended = false;

        let hwnd = match raw_window_handle.raw_window_handle() {
            RawWindowHandle::Win32(hwnd) => {
                println!("HWND: {:?}", hwnd);
                windows::Win32::Foundation::HWND(hwnd.hwnd as isize)
            }
            _ => return Err("Cannot create RafxSurfaceDx12 on this operating system".into()),
        };

        let sample_desc = super::dxgi::Common::DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        };

        let desc = dxgi::DXGI_SWAP_CHAIN_DESC1 {
            Width: swapchain_def.width,
            Height: swapchain_def.height,
            Format: pixel_format,
            Stereo: windows::Win32::Foundation::FALSE,
            SampleDesc: sample_desc,
            BufferUsage: dxgi::DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: SWAPCHAIN_IMAGE_COUNT,
            Scaling: dxgi::DXGI_SCALING_STRETCH,
            SwapEffect: dxgi::DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: dxgi::Common::DXGI_ALPHA_MODE_UNSPECIFIED,
            Flags: 0,
        };

        //TODO: vsync (i.e. allow tearing)
        let swapchain = unsafe {
            //TODO: First param should be a queue
            let swapchain: IDXGISwapChain3 = device_context
                .dxgi_factory()
                .CreateSwapChainForHwnd(present_queue.dx12_queue(), hwnd, &desc, None, None)?
                .cast()?;
            device_context
                .dxgi_factory()
                .MakeWindowAssociation(hwnd, dxgi::DXGI_MWA_NO_ALT_ENTER)?;
            swapchain
        };

        let swapchain_images = Self::create_swapchain_images(
            device_context,
            &swapchain_def,
            swapchain_format,
            &swapchain,
        )?;

        let swapchain_def = swapchain_def.clone();

        Ok(RafxSwapchainDx12 {
            device_context: device_context.clone(),
            swapchain_def,
            format: swapchain_format,
            color_space: preferred_color_space,
            swapchain_images,
            swapchain,
            swapchain_flags: dxgi::DXGI_SWAP_CHAIN_FLAG(0),
            present_queue: present_queue.clone(),
        })
    }

    fn create_swapchain_images(
        device_context: &RafxDeviceContextDx12,
        swapchain_def: &&RafxSwapchainDef,
        swapchain_format: RafxFormat,
        swapchain: &IDXGISwapChain3,
    ) -> RafxResult<Vec<RafxSwapchainImage>> {
        let mut swapchain_images = Vec::with_capacity(SWAPCHAIN_IMAGE_COUNT as usize);

        // create images
        for i in 0..SWAPCHAIN_IMAGE_COUNT {
            let raw_image = RafxRawImageDx12 {
                image: unsafe { swapchain.GetBuffer(i)? },
                allocation: None,
            };
            let format = swapchain_format;
            let resource_type = RafxResourceType::TEXTURE | RafxResourceType::RENDER_TARGET_COLOR;

            let swapchain_image = RafxTextureDx12::from_existing(
                device_context,
                Some(raw_image),
                &RafxTextureDef {
                    extents: RafxExtents3D {
                        width: swapchain_def.width,
                        height: swapchain_def.height,
                        depth: 1,
                    },
                    array_length: 1,
                    mip_count: 1,
                    format,
                    resource_type,
                    sample_count: RafxSampleCount::SampleCount1,
                    dimensions: RafxTextureDimensions::Dim2D,
                },
            )?;

            swapchain_images.push(RafxSwapchainImage {
                texture: RafxTexture::Dx12(swapchain_image),
                swapchain_image_index: i,
            });
        }
        Ok(swapchain_images)
    }

    pub fn rebuild(
        &mut self,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<()> {
        // wait for idle
        self.present_queue.wait_for_queue_idle()?;

        // release images
        self.swapchain_images.clear();

        // set frame fence events

        // release swapchain buffers
        //TODO: Select format
        //let preferred_color_space = RafxSwapchainColorSpace::Srgb;
        let pixel_format = dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
        let swapchain_format = RafxFormat::B8G8R8A8_SRGB;
        //let is_extended = false;
        unsafe {
            self.swapchain.ResizeBuffers(
                SWAPCHAIN_IMAGE_COUNT,
                swapchain_def.width,
                swapchain_def.height,
                pixel_format,
                0,
            )?;
        }

        let swapchain_images = Self::create_swapchain_images(
            &self.device_context,
            &swapchain_def,
            swapchain_format,
            &self.swapchain,
        )?;
        self.swapchain_images = swapchain_images;

        self.swapchain_def = swapchain_def.clone();
        Ok(())
    }

    pub fn acquire_next_image_fence(
        &mut self,
        _fence: &RafxFenceDx12,
    ) -> RafxResult<RafxSwapchainImage> {
        self.acquire_next_image()
    }

    pub fn acquire_next_image_semaphore(
        &mut self,
        _semaphore: &RafxSemaphoreDx12,
    ) -> RafxResult<RafxSwapchainImage> {
        self.acquire_next_image()
    }

    pub fn acquire_next_image(&mut self) -> RafxResult<RafxSwapchainImage> {
        let swapchain_image_index = unsafe { self.swapchain.GetCurrentBackBufferIndex() };
        Ok(self.swapchain_images[swapchain_image_index as usize].clone())
    }
}
