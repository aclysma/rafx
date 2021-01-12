use crate::game_renderer::swapchain_resources::SwapchainResources;
use crate::game_renderer::GameRenderer;
use legion::Resources;
use rafx::api::{
    RafxDeviceContext, RafxExtents2D, RafxPresentableFrame, RafxResult, RafxSwapchain,
    RafxSwapchainDef, RafxSwapchainEventListener, RafxSwapchainHelper,
};
use rafx::assets::AssetManager;
use rafx::nodes::RenderRegistry;
use rafx::raw_window_handle::HasRawWindowHandle;
use rafx::resources::graph::SwapchainSurfaceInfo;

pub struct SwapchainHandler<'a> {
    pub resources: &'a Resources,
    pub asset_manager: &'a mut AssetManager,
    pub render_registry: &'a RenderRegistry,
    pub game_renderer: &'a GameRenderer,
}

impl<'a> SwapchainHandler<'a> {
    #[profiling::function]
    pub fn create_swapchain(
        resources: &mut Resources,
        window: &dyn HasRawWindowHandle,
        width: u32,
        height: u32,
    ) -> RafxResult<()> {
        let swapchain_helper = {
            let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
            let render_registry = resources.get::<RenderRegistry>().unwrap();
            let mut game_renderer = resources.get_mut::<GameRenderer>().unwrap();

            let mut lifetime_listener = SwapchainHandler {
                resources: &resources,
                asset_manager: &mut *asset_manager,
                render_registry: &*render_registry,
                game_renderer: &mut *game_renderer,
            };

            let device_context = &*resources.get::<RafxDeviceContext>().unwrap();
            let swapchain = device_context.create_swapchain(
                window,
                &RafxSwapchainDef {
                    height,
                    width,
                    enable_vsync: true,
                },
            )?;

            rafx::api::RafxSwapchainHelper::new(
                device_context,
                swapchain,
                Some(&mut lifetime_listener),
            )?
        };

        resources.insert(swapchain_helper);

        Ok(())
    }

    #[profiling::function]
    pub fn acquire_next_image(
        resources: &Resources,
        window_width: u32,
        window_height: u32,
        game_renderer: &GameRenderer,
    ) -> RafxResult<RafxPresentableFrame> {
        let mut surface = resources.get_mut::<RafxSwapchainHelper>().unwrap();
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();

        let mut lifetime_listener = SwapchainHandler {
            resources: &resources,
            asset_manager: &mut *asset_manager,
            render_registry: &*render_registry,
            game_renderer,
        };

        surface.acquire_next_image(window_width, window_height, Some(&mut lifetime_listener))
    }

    #[profiling::function]
    pub fn destroy_swapchain(resources: &mut Resources) -> RafxResult<()> {
        let mut surface = resources.remove::<RafxSwapchainHelper>().unwrap();
        let mut game_renderer = resources.get_mut::<GameRenderer>().unwrap();
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();

        let mut lifetime_listener = SwapchainHandler {
            resources: &resources,
            asset_manager: &mut *asset_manager,
            render_registry: &*render_registry,
            game_renderer: &mut game_renderer,
        };

        surface.destroy(Some(&mut lifetime_listener))?;
        std::mem::drop(surface);
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
        let mut guard = self.game_renderer.inner.lock().unwrap();
        let mut game_renderer = &mut *guard;
        let asset_manager = &mut self.asset_manager;

        //
        // Metadata about the swapchain
        //
        log::debug!("game renderer swapchain_created called");

        let swapchain_def = swapchain.swapchain_def();
        let extents = RafxExtents2D {
            width: swapchain_def.width,
            height: swapchain_def.height,
        };

        let swapchain_surface_info = SwapchainSurfaceInfo {
            extents,
            format: swapchain.format(),
        };

        //
        // Construct resources that are tied to the swapchain or swapchain metadata.
        // (i.e. renderpasses, descriptor sets that refer to swapchain images)
        //
        let swapchain_resources = SwapchainResources::new(
            device_context,
            swapchain,
            game_renderer,
            asset_manager.resource_manager_mut(),
            swapchain_surface_info,
        )?;

        game_renderer.swapchain_resources = Some(swapchain_resources);

        log::debug!("game renderer swapchain_created finished");

        Ok(())
    }

    #[profiling::function]
    fn swapchain_destroyed(
        &mut self,
        _device_context: &RafxDeviceContext,
        _swapchain: &RafxSwapchain,
    ) -> RafxResult<()> {
        let mut guard = self.game_renderer.inner.lock().unwrap();
        let game_renderer = &mut *guard;

        log::debug!("game renderer swapchain destroyed");

        // This will clear game_renderer.swapchain_resources and drop SwapchainResources at end of fn
        let swapchain_resources = game_renderer.swapchain_resources.take().unwrap();
        std::mem::drop(swapchain_resources);

        //TODO: Explicitly remove the images instead of just dropping them. This prevents anything
        // from accidentally using them after they've been freed
        //swapchain_resources.swapchain_images.clear();

        // self.resource_manager
        //     .remove_swapchain(&swapchain_resources.swapchain_surface_info);

        Ok(())
    }
}
