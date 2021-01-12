use crate::metal::RafxPresentableFrameMetal;
use crate::RafxResult;
use metal::CGSize;
use raw_window_handle::HasRawWindowHandle;

// pub struct RafxPresentableFrameMetal {
//     drawable: metal::CoreAnimationDrawable
// }
//
// impl RafxPresentableFrameMetal {
//     pub fn cancel_present(self, result: RafxError) {
//
//     }
//
//     pub fn present(self, /* command buffers? */) {
//         // TODO: wait for something?
//         self.drawable.present();
//     }
// }

/*
impl FrameInFlight {
    // A value that stays in step with the image index returned by the swapchain. There is no
    // guarantee on the ordering of present image index (i.e. it may decrease). It is only promised
    // to not be in use by a frame in flight
    pub fn present_index(&self) -> u32;

    // If true, consider recreating the swapchain
    pub fn is_suboptimal(&self) -> bool;

    // Can be called by the end user to end the frame early and defer a result to the next acquire
    // image call
    pub fn cancel_present(
        self,
        result: VkResult<()>,
    );

    // submit the given command buffers and preset the swapchain image for this frame
    pub fn present(
        self,
        command_buffers: &[vk::CommandBuffer],
    ) -> VkResult<()>;
}
*/

pub struct RafxSurfaceMetal {
    layer: metal::CoreAnimationLayer,
}

impl RafxSurfaceMetal {
    pub fn new(
        device: &metal::Device,
        raw_window_handle: &dyn HasRawWindowHandle,
        width: u32,
        height: u32,
    ) -> RafxResult<Self> {
        let layer = match raw_window_handle.raw_window_handle() {
            #[cfg(target_os = "macos")]
            raw_window_handle::RawWindowHandle::MacOS(handle) => unsafe {
                raw_window_metal::macos::metal_layer_from_handle(handle)
            },
            #[cfg(target_os = "ios")]
            raw_window_handle::RawWindowHandle::IOS(handle) => unsafe {
                raw_window_metal::ios::metal_layer_from_handle(handle)
            },
            _ => return Err("Cannot create RafxSurfaceMetal on this operating system".into()),
        };

        let layer = match layer {
            raw_window_metal::Layer::Allocated(x) => Some(x),
            raw_window_metal::Layer::Existing(x) => Some(x),
            raw_window_metal::Layer::None => None,
        }
        .unwrap();

        let layer =
            unsafe { std::mem::transmute::<_, &metal::CoreAnimationLayerRef>(layer).to_owned() };

        layer.set_device(device);
        layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);

        //TODO: disable timeout on acquire drawable?

        layer.set_drawable_size(metal::CGSize::new(width as f64, height as f64));

        Ok(RafxSurfaceMetal { layer })
    }

    pub fn layer(&self) -> &metal::CoreAnimationLayer {
        &self.layer
    }

    pub fn begin_frame(
        &self,
        window_width: u32,
        window_height: u32,
    ) -> RafxResult<RafxPresentableFrameMetal> {
        let drawable = self
            .layer
            .next_drawable()
            .ok_or("Timed out while trying to acquire drawable".to_string())?
            .to_owned();

        self.layer.set_drawable_size(CGSize {
            width: window_width as f64,
            height: window_height as f64,
        });

        Ok(RafxPresentableFrameMetal::new(drawable))
    }
}

/*



/// May be implemented to get callbacks related to the swapchain being created/destroyed
pub trait VkSurfaceSwapchainLifetimeListener {
    /// Called whenever the swapchain needs to be created (the first time, and in cases where the
    /// swapchain needs to be recreated)
    fn swapchain_created(
        &mut self,
        device_context: &RafxDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()>;

    /// Called whenever the swapchain will be destroyed (when VkSurface is dropped, and also in cases
    /// where the swapchain needs to be recreated)
    fn swapchain_destroyed(
        &mut self,
        device_context: &RafxDeviceContext,
        swapchain: &VkSwapchain,
    );
}


impl VkSurface {
    /// Create the surface - a per-window object that maintains the swapchain
    pub fn new(
        device_context: &RafxDeviceContext,
        window_inner_size: Extent2D,
        event_listener: Option<&mut dyn VkSurfaceSwapchainLifetimeListener>,
        present_mode_priority: Option<&[PresentMode]>
    ) -> VkResult<VkSurface>;

    pub fn tear_down(
        &mut self,
        event_listener: Option<&mut dyn VkSurfaceSwapchainLifetimeListener>,
    );

    // If a frame is in flight, block until it completes
    pub fn wait_until_frame_not_in_flight(&self) -> VkResult<()>;

    pub fn acquire_next_swapchain_image(
        &mut self,
        window_inner_size: Extent2D
    ) -> VkResult<FrameInFlight>;

    pub fn rebuild_swapchain(
        &mut self,
        window_inner_size: Extent2D,
        mut event_listener: Option<&mut dyn VkSurfaceSwapchainLifetimeListener>,
    ) -> VkResult<()>;
}

*/
