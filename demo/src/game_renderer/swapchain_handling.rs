use renderer::vulkan::{
    VkContext, VkSurface, Window, VkSurfaceSwapchainLifetimeListener, VkDeviceContext, VkSwapchain,
};
use crate::game_renderer::GameRenderer;
use legion::prelude::Resources;
use ash::prelude::VkResult;
use renderer::resources::resource_managers::ResourceManager;
use renderer::nodes::RenderRegistry;
use crate::game_renderer::swapchain_resources::SwapchainResources;
use renderer::assets::vk_description::SwapchainSurfaceInfo;

pub struct SwapchainLifetimeListener<'a> {
    pub resources: &'a Resources,
    pub resource_manager: &'a mut ResourceManager,
    pub render_registry: &'a RenderRegistry,
    pub game_renderer: &'a GameRenderer,
}

impl<'a> SwapchainLifetimeListener<'a> {
    pub fn create_surface(
        resources: &Resources,
        window: &dyn Window,
    ) -> VkResult<VkSurface> {
        let mut resource_manager = resources.get_mut::<ResourceManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();
        let mut game_renderer = resources.get_mut::<GameRenderer>().unwrap();

        let mut lifetime_listener = SwapchainLifetimeListener {
            resources: &resources,
            resource_manager: &mut *resource_manager,
            render_registry: &*render_registry,
            game_renderer: &mut *game_renderer,
        };

        VkSurface::new(
            &*resources.get::<VkContext>().unwrap(),
            window,
            Some(&mut lifetime_listener),
        )
    }

    pub fn rebuild_swapchain(
        resources: &Resources,
        window: &dyn Window,
        game_renderer: &GameRenderer,
    ) -> VkResult<()> {
        let mut surface = resources.get_mut::<VkSurface>().unwrap();
        let mut resource_manager = resources.get_mut::<ResourceManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();

        let mut lifetime_listener = SwapchainLifetimeListener {
            resources: &resources,
            resource_manager: &mut *resource_manager,
            render_registry: &*render_registry,
            game_renderer,
        };

        surface.rebuild_swapchain(window, &mut Some(&mut lifetime_listener))
    }

    pub fn tear_down(resources: &Resources) {
        let mut surface = resources.get_mut::<VkSurface>().unwrap();
        let mut game_renderer = resources.get_mut::<GameRenderer>().unwrap();
        let mut resource_manager = resources.get_mut::<ResourceManager>().unwrap();
        let render_registry = resources.get::<RenderRegistry>().unwrap();

        let mut lifetime_listener = SwapchainLifetimeListener {
            resources: &resources,
            resource_manager: &mut *resource_manager,
            render_registry: &*render_registry,
            game_renderer: &mut game_renderer,
        };

        surface.tear_down(Some(&mut lifetime_listener));
    }
}

impl<'a> VkSurfaceSwapchainLifetimeListener for SwapchainLifetimeListener<'a> {
    fn swapchain_created(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) -> VkResult<()> {
        let mut guard = self.game_renderer.inner.lock().unwrap();
        let mut game_renderer = &mut *guard;
        let mut resource_manager = &mut self.resource_manager;

        log::debug!("game renderer swapchain_created called");
        let swapchain_surface_info = SwapchainSurfaceInfo {
            extents: swapchain.swapchain_info.extents,
            msaa_level: swapchain.swapchain_info.msaa_level,
            surface_format: swapchain.swapchain_info.surface_format,
            color_format: swapchain.color_format,
            depth_format: swapchain.depth_format,
        };

        resource_manager.add_swapchain(&swapchain_surface_info);

        let swapchain_resources = SwapchainResources::new(
            device_context,
            swapchain,
            game_renderer,
            resource_manager,
            swapchain_surface_info,
        )?;

        game_renderer.swapchain_resources = Some(swapchain_resources);

        log::debug!("game renderer swapchain_created finished");

        VkResult::Ok(())
    }

    fn swapchain_destroyed(
        &mut self,
        device_context: &VkDeviceContext,
        swapchain: &VkSwapchain,
    ) {
        let mut guard = self.game_renderer.inner.lock().unwrap();
        let mut game_renderer = &mut *guard;

        log::debug!("game renderer swapchain destroyed");

        let swapchain_surface_info = SwapchainSurfaceInfo {
            extents: swapchain.swapchain_info.extents,
            msaa_level: swapchain.swapchain_info.msaa_level,
            surface_format: swapchain.swapchain_info.surface_format,
            color_format: swapchain.color_format,
            depth_format: swapchain.depth_format,
        };

        // This will clear game_renderer.swapchain_resources and drop SwapchainResources at end of fn
        let swapchain_resources = game_renderer.swapchain_resources.take().unwrap();

        self.resource_manager
            .remove_swapchain(&swapchain_resources.swapchain_surface_info);
    }
}
