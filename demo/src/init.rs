use legion::Resources;
use rafx::api::{RafxApi, RafxDeviceContext, RafxResult, RafxSwapchainHelper};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::framework::visibility::VisibilityRegion;
use rafx::render_features::{ExtractResources, RenderRegistry};
use rafx::renderer::{
    AssetSource, Renderer, RendererBuilder, RendererConfigResource, SwapchainHandler,
    ViewportsResource,
};
use rafx_plugins::assets::anim::AnimAssetTypeRendererPlugin;
use rafx_plugins::assets::font::FontAssetTypeRendererPlugin;
use rafx_plugins::assets::ldtk::LdtkAssetTypeRendererPlugin;
use rafx_plugins::assets::mesh_basic::GltfAssetTypeRendererPlugin;
use rafx_plugins::features::debug3d::Debug3DRendererPlugin;
use rafx_plugins::features::mesh_basic::MeshBasicRendererPlugin;
use rafx_plugins::features::skybox::SkyboxRendererPlugin;
use rafx_plugins::features::sprite::SpriteRendererPlugin;
use rafx_plugins::features::text::TextRendererPlugin;
use rafx_plugins::features::tile_layer::TileLayerRendererPlugin;
use rafx_plugins::pipelines::basic::BasicPipelineRendererPlugin;
use rafx_plugins::pipelines::basic::BasicRenderGraphGenerator;
use raw_window_handle::HasRawWindowHandle;
use std::sync::Arc;

pub fn rendering_init(
    resources: &mut Resources,
    asset_source: AssetSource,
    window: &dyn HasRawWindowHandle,
    window_width: u32,
    window_height: u32,
) -> RafxResult<()> {
    resources.insert(VisibilityRegion::new());
    resources.insert(ViewportsResource::default());

    let mesh_renderer_plugin = Arc::new(MeshBasicRendererPlugin::new(Some(32)));
    let sprite_renderer_plugin = Arc::new(SpriteRendererPlugin::default());
    let skybox_renderer_plugin = Arc::new(SkyboxRendererPlugin::default());
    let tile_layer_renderer_plugin = Arc::new(TileLayerRendererPlugin::default());
    let debug3d_renderer_plugin = Arc::new(Debug3DRendererPlugin::default());
    let text_renderer_plugin = Arc::new(TextRendererPlugin::default());

    #[cfg(feature = "egui")]
    let egui_renderer_plugin =
        Arc::new(rafx_plugins::features::egui::EguiRendererPlugin::default());
    mesh_renderer_plugin.legion_init(resources);
    sprite_renderer_plugin.legion_init(resources);
    skybox_renderer_plugin.legion_init(resources);
    tile_layer_renderer_plugin.legion_init(resources);
    debug3d_renderer_plugin.legion_init(resources);
    text_renderer_plugin.legion_init(resources);

    #[cfg(feature = "egui")]
    egui_renderer_plugin.legion_init_winit(resources);

    //
    // Create the api. GPU programming is fundamentally unsafe, so all rafx APIs should be
    // considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    // behavior on the CPU for reasons other than interacting with the GPU.
    //
    let rafx_api = unsafe { rafx::api::RafxApi::new(window, &Default::default())? };

    let allow_use_render_thread = if cfg!(feature = "stats_alloc") {
        false
    } else {
        true
    };

    let mut renderer_builder = RendererBuilder::default();
    renderer_builder = renderer_builder
        .add_asset(Arc::new(FontAssetTypeRendererPlugin))
        .add_asset(Arc::new(GltfAssetTypeRendererPlugin))
        .add_asset(Arc::new(LdtkAssetTypeRendererPlugin))
        .add_asset(Arc::new(AnimAssetTypeRendererPlugin))
        .add_asset(Arc::new(BasicPipelineRendererPlugin))
        .add_render_feature(mesh_renderer_plugin)
        .add_render_feature(sprite_renderer_plugin)
        .add_render_feature(skybox_renderer_plugin)
        .add_render_feature(tile_layer_renderer_plugin)
        .add_render_feature(debug3d_renderer_plugin)
        .add_render_feature(text_renderer_plugin)
        .allow_use_render_thread(allow_use_render_thread);

    #[cfg(feature = "egui")]
    {
        renderer_builder = renderer_builder.add_render_feature(egui_renderer_plugin)
    }

    let mut renderer_builder_result = {
        let extract_resources = ExtractResources::default();

        let render_graph_generator = Box::new(BasicRenderGraphGenerator);

        renderer_builder.build(
            extract_resources,
            &rafx_api,
            asset_source,
            render_graph_generator,
            || {
                None
                // Some(Box::new(DemoRendererThreadPool::new()))
            },
        )
    }?;

    let swapchain_helper = SwapchainHandler::create_swapchain(
        &mut renderer_builder_result.asset_manager,
        &mut renderer_builder_result.renderer,
        window,
        window_width,
        window_height,
    )?;

    resources.insert(rafx_api.device_context());
    resources.insert(rafx_api);
    resources.insert(swapchain_helper);
    resources.insert(renderer_builder_result.asset_resource);
    resources.insert(
        renderer_builder_result
            .asset_manager
            .resource_manager()
            .render_registry()
            .clone(),
    );
    resources.insert(renderer_builder_result.asset_manager);
    resources.insert(renderer_builder_result.renderer);
    resources.insert(RendererConfigResource::default());

    Ok(())
}

pub fn rendering_destroy(resources: &mut Resources) -> RafxResult<()> {
    // Destroy these first
    {
        {
            let swapchain_helper = resources.remove::<RafxSwapchainHelper>().unwrap();
            let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
            let renderer = resources.get::<Renderer>().unwrap();
            SwapchainHandler::destroy_swapchain(swapchain_helper, &mut *asset_manager, &*renderer)?;
        }

        resources.remove::<Renderer>();

        MeshBasicRendererPlugin::legion_destroy(resources);
        SpriteRendererPlugin::legion_destroy(resources);
        SkyboxRendererPlugin::legion_destroy(resources);
        TileLayerRendererPlugin::legion_destroy(resources);
        Debug3DRendererPlugin::legion_destroy(resources);
        TextRendererPlugin::legion_destroy(resources);

        #[cfg(feature = "egui")]
        {
            rafx_plugins::features::egui::EguiRendererPlugin::legion_destroy(resources);
        }

        resources.remove::<RenderRegistry>();

        // Remove the asset resource because we have asset storages that reference resources
        resources.remove::<AssetResource>();

        resources.remove::<AssetManager>();
        resources.remove::<RafxDeviceContext>();
    }

    // Drop this one last
    resources.remove::<RafxApi>();
    Ok(())
}
