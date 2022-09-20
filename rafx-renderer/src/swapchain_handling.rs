use super::swapchain_render_resource::SwapchainRenderResource;
use super::Renderer;
use rafx_api::raw_window_handle::HasRawWindowHandle;
use rafx_api::{
    RafxDeviceContext, RafxExtents2D, RafxPresentableFrame, RafxResult, RafxSwapchain,
    RafxSwapchainDef, RafxSwapchainEventListener, RafxSwapchainHelper,
};
use rafx_assets::AssetManager;
use rafx_framework::graph::SwapchainSurfaceInfo;

pub struct SwapchainHandler<'a> {
    pub asset_manager: &'a mut AssetManager,
    pub renderer: &'a Renderer,
}

impl<'a> SwapchainHandler<'a> {
    #[profiling::function]
    pub fn create_swapchain(
        asset_manager: &mut AssetManager,
        renderer: &mut Renderer,
        window: &dyn HasRawWindowHandle,
        swapchain_def: &RafxSwapchainDef,
    ) -> RafxResult<RafxSwapchainHelper> {
        let swapchain_helper = {
            let device_context = asset_manager.device_context().clone();
            let swapchain = device_context.create_swapchain(window, swapchain_def)?;

            let mut lifetime_listener = SwapchainHandler {
                asset_manager,
                renderer,
            };

            RafxSwapchainHelper::new(&device_context, swapchain, Some(&mut lifetime_listener))?
        };

        Ok(swapchain_helper)
    }

    #[profiling::function]
    pub fn acquire_next_image(
        swapchain_helper: &mut RafxSwapchainHelper,
        asset_manager: &mut AssetManager,
        renderer: &Renderer,
        window_width: u32,
        window_height: u32,
    ) -> RafxResult<RafxPresentableFrame> {
        let mut lifetime_listener = SwapchainHandler {
            asset_manager,
            renderer,
        };

        swapchain_helper.acquire_next_image(
            window_width,
            window_height,
            Some(&mut lifetime_listener),
        )
    }

    #[profiling::function]
    pub fn destroy_swapchain(
        mut swapchain_helper: RafxSwapchainHelper,
        asset_manager: &mut AssetManager,
        renderer: &Renderer,
    ) -> RafxResult<()> {
        let mut lifetime_listener = SwapchainHandler {
            asset_manager,
            renderer,
        };

        swapchain_helper.destroy(Some(&mut lifetime_listener))?;
        std::mem::drop(swapchain_helper);
        Ok(())
    }
}

impl<'a> RafxSwapchainEventListener for SwapchainHandler<'a> {
    #[profiling::function]
    fn swapchain_created(
        &mut self,
        device_context: &RafxDeviceContext,
        swapchain: &RafxSwapchain,
    ) -> RafxResult<()> {
        //
        // Metadata about the swapchain
        //
        log::debug!("renderer swapchain_created called");

        let swapchain_def = swapchain.swapchain_def();
        let extents = RafxExtents2D {
            width: swapchain_def.width,
            height: swapchain_def.height,
        };

        let swapchain_surface_info = SwapchainSurfaceInfo {
            extents,
            format: swapchain.format(),
            color_space: swapchain.color_space(),
        };

        //
        // Construct resources that are tied to the swapchain or swapchain metadata.
        // (i.e. renderpasses, descriptor sets that refer to swapchain images)
        //
        let mut swapchain_render_resource = self
            .renderer
            .render_resources
            .fetch_mut::<SwapchainRenderResource>();
        swapchain_render_resource.set_swapchain_info(device_context, swapchain_surface_info)?;

        log::debug!("renderer swapchain_created finished");

        Ok(())
    }

    #[profiling::function]
    fn swapchain_destroyed(
        &mut self,
        _device_context: &RafxDeviceContext,
        _swapchain: &RafxSwapchain,
    ) -> RafxResult<()> {
        log::debug!("renderer swapchain destroyed");

        let mut swapchain_render_resource = self
            .renderer
            .render_resources
            .fetch_mut::<SwapchainRenderResource>();
        swapchain_render_resource.clear_swapchain_info();

        //TODO: Explicitly remove the images instead of just dropping them. This prevents anything
        // from accidentally using them after they've been freed
        //swapchain_render_resource.swapchain_images.clear();

        // self.resource_manager
        //     .remove_swapchain(&swapchain_render_resource.swapchain_surface_info);

        Ok(())
    }
}
