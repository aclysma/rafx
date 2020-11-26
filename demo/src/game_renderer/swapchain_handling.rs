use crate::game_renderer::swapchain_resources::SwapchainResources;
use crate::game_renderer::GameRenderer;
use ash::prelude::VkResult;
use legion::Resources;
use renderer::assets::AssetManager;
use renderer::nodes::RenderRegistry;
use renderer::resources::vk_description as dsc;
use renderer::vulkan::{
    VkContext, VkDeviceContext, VkSurface, VkSurfaceSwapchainLifetimeListener, VkSwapchain, Window,
};

pub struct SwapchainLifetimeListener<'a> {
    pub resources: &'a Resources,
    pub asset_manager: &'a mut AssetManager,
    pub render_registry: &'a RenderRegistry,
    pub game_renderer: &'a GameRenderer,
}

impl<'a> SwapchainLifetimeListener<'a> {
    #[profiling::function]
    pub fn create_surface(
        resources: &Resources,
        window: &dyn Window,
    ) -> VkResult<VkSurface> {
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();
        let mut game_renderer = resources.get_mut::<GameRenderer>().unwrap();

        let mut lifetime_listener = SwapchainLifetimeListener {
            resources: &resources,
            asset_manager: &mut *asset_manager,
            render_registry: &*render_registry,
            game_renderer: &mut *game_renderer,
        };

        VkSurface::new(
            &*resources.get::<VkContext>().unwrap(),
            window,
            Some(&mut lifetime_listener),
        )
    }

    #[profiling::function]
    pub fn rebuild_swapchain(
        resources: &Resources,
        window: &dyn Window,
        game_renderer: &GameRenderer,
    ) -> VkResult<()> {
        let mut surface = resources.get_mut::<VkSurface>().unwrap();
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();

        let mut lifetime_listener = SwapchainLifetimeListener {
            resources: &resources,
            asset_manager: &mut *asset_manager,
            render_registry: &*render_registry,
            game_renderer,
        };

        surface.rebuild_swapchain(window, Some(&mut lifetime_listener))
    }

    #[profiling::function]
    pub fn tear_down(resources: &Resources) {
        let mut surface = resources.get_mut::<VkSurface>().unwrap();
        let mut game_renderer = resources.get_mut::<GameRenderer>().unwrap();
        let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();

        let mut lifetime_listener = SwapchainLifetimeListener {
            resources: &resources,
            asset_manager: &mut *asset_manager,
            render_registry: &*render_registry,
            game_renderer: &mut game_renderer,
        };

        surface.tear_down(Some(&mut lifetime_listener));
    }
}

impl<'a> VkSurfaceSwapchainLifetimeListener for SwapchainLifetimeListener<'a> {
    #[profiling::function]
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()> {
        let mut guard = self.game_renderer.inner.lock().unwrap();
        let mut game_renderer = &mut *guard;
        let asset_manager = &mut self.asset_manager;

        //
        // Metadata about the swapchain
        //
        log::debug!("game renderer swapchain_created called");
        let swapchain_surface_info = dsc::SwapchainSurfaceInfo {
            extents: swapchain.swapchain_info.extents,
            msaa_level: swapchain.swapchain_info.msaa_level,
            surface_format: swapchain.swapchain_info.surface_format,
            color_format: swapchain.color_format,
            depth_format: swapchain.depth_format,
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
            swapchain.swapchain_info.clone(),
            swapchain_surface_info,
        )?;

        game_renderer.swapchain_resources = Some(swapchain_resources);

        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(())
    }

    #[profiling::function]
    fn swapchain_destroyed(
        &mut self,
        _device_context: &VkDeviceContext,
        _swapchain: &VkSwapchain,
    ) {
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
    }
}
