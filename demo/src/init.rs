use legion::Resources;
use rafx::api::{
    RafxApi, RafxApiDef, RafxDeviceContext, RafxResult, RafxSwapchainColorSpace, RafxSwapchainDef,
    RafxSwapchainHelper,
};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::framework::visibility::VisibilityResource;
use rafx::render_features::{ExtractResources, RenderRegistry};
use rafx::renderer::{
    AssetSource, Renderer, RendererBuilder, RendererConfigResource, SwapchainHandler,
    ViewportsResource,
};
use rafx_plugins::assets::anim::AnimAssetTypeRendererPlugin;
use rafx_plugins::assets::font::FontAssetTypeRendererPlugin;
use rafx_plugins::assets::ldtk::LdtkAssetTypeRendererPlugin;
use rafx_plugins::features::debug3d::Debug3DRendererPlugin;
use rafx_plugins::features::debug_pip::DebugPipRendererPlugin;
use rafx_plugins::features::skybox::SkyboxRendererPlugin;
use rafx_plugins::features::sprite::SpriteRendererPlugin;
use rafx_plugins::features::text::TextRendererPlugin;
use rafx_plugins::features::tile_layer::TileLayerRendererPlugin;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::sync::Arc;

#[cfg(feature = "rafx-metal")]
use rafx::api::metal::RafxApiDefMetal;

#[cfg(feature = "rafx-vulkan")]
use rafx::api::vulkan::RafxApiDefVulkan;

#[cfg(feature = "basic-pipeline")]
use rafx_plugins::assets::mesh_basic::MeshBasicAssetTypeRendererPlugin;
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::features::mesh_basic::MeshBasicRendererPlugin;
#[cfg(feature = "basic-pipeline")]
use rafx_plugins::pipelines::basic::BasicPipelineRendererPlugin;

#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::assets::mesh_adv::MeshAdvAssetTypeRendererPlugin;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::features::mesh_adv::MeshAdvRendererPlugin;
#[cfg(not(feature = "basic-pipeline"))]
use rafx_plugins::pipelines::modern::ModernPipelineRendererPlugin;

pub fn rendering_init(
    resources: &mut Resources,
    asset_source: AssetSource,
    display: &dyn HasRawDisplayHandle,
    window: &dyn HasRawWindowHandle,
    window_width: u32,
    window_height: u32,
) -> RafxResult<()> {
    resources.insert(VisibilityResource::new());
    resources.insert(ViewportsResource::default());

    #[cfg(feature = "basic-pipeline")]
    let mesh_renderer_plugin = Arc::new(MeshBasicRendererPlugin::new(Some(32)));
    #[cfg(not(feature = "basic-pipeline"))]
    let mesh_renderer_plugin = Arc::new(MeshAdvRendererPlugin::new(Some(32)));
    let sprite_renderer_plugin = Arc::new(SpriteRendererPlugin::default());
    let skybox_renderer_plugin = Arc::new(SkyboxRendererPlugin::default());
    let tile_layer_renderer_plugin = Arc::new(TileLayerRendererPlugin::default());
    let debug3d_renderer_plugin = Arc::new(Debug3DRendererPlugin::default());
    let debug_pip_renderer_plugin = Arc::new(DebugPipRendererPlugin::default());
    let text_renderer_plugin = Arc::new(TextRendererPlugin::default());

    #[cfg(feature = "egui")]
    let egui_renderer_plugin =
        Arc::new(rafx_plugins::features::egui::EguiRendererPlugin::default());
    mesh_renderer_plugin.legion_init(resources);
    sprite_renderer_plugin.legion_init(resources);
    skybox_renderer_plugin.legion_init(resources);
    tile_layer_renderer_plugin.legion_init(resources);
    debug3d_renderer_plugin.legion_init(resources);
    debug_pip_renderer_plugin.legion_init(resources);
    text_renderer_plugin.legion_init(resources);

    #[cfg(feature = "egui")]
    egui_renderer_plugin.legion_init_winit(resources);

    //
    // Create the api. GPU programming is fundamentally unsafe, so all rafx APIs should be
    // considered unsafe. However, rafx APIs are only gated by unsafe if they can cause undefined
    // behavior on the CPU for reasons other than interacting with the GPU.
    //

    #[allow(unused_mut)]
    let mut api_def = RafxApiDef::default();

    // Turn on debug names for the demo
    #[cfg(feature = "rafx-metal")]
    {
        let mut options = RafxApiDefMetal::default();
        options.enable_debug_names = true;
        api_def.metal_options = Some(options);
    }

    #[cfg(feature = "rafx-vulkan")]
    {
        let mut options = RafxApiDefVulkan::default();
        options.enable_debug_names = true;

        // For vulkan on the modern pipeline, we need to enable shader_clip_distance. The default-enabled
        // options in rafx-api are fine for the basic pipeline
        #[cfg(not(feature = "basic-pipeline"))]
        {
            let physical_device_features = rafx::api::ash::vk::PhysicalDeviceFeatures::builder()
                .sampler_anisotropy(true)
                .sample_rate_shading(true)
                // Used for debug drawing lines/points
                .fill_mode_non_solid(true)
                // Used for user clipping in shadow atlas generation
                .shader_clip_distance(true)
                // Used for indirect draw
                .multi_draw_indirect(true)
                .draw_indirect_first_instance(true)
                .build();

            options.physical_device_features = Some(physical_device_features);
        }

        api_def.vk_options = Some(options);
    }

    let rafx_api = unsafe { rafx::api::RafxApi::new(display, window, &api_def)? };

    let allow_use_render_thread = if cfg!(feature = "stats_alloc") {
        false
    } else {
        true
    };

    let mut renderer_builder = RendererBuilder::default();
    renderer_builder = renderer_builder
        .add_asset_plugin(Arc::new(FontAssetTypeRendererPlugin))
        .add_asset_plugin(Arc::new(LdtkAssetTypeRendererPlugin))
        .add_asset_plugin(Arc::new(AnimAssetTypeRendererPlugin))
        .add_render_feature_plugin(mesh_renderer_plugin)
        .add_render_feature_plugin(sprite_renderer_plugin)
        .add_render_feature_plugin(skybox_renderer_plugin)
        .add_render_feature_plugin(tile_layer_renderer_plugin)
        .add_render_feature_plugin(debug3d_renderer_plugin)
        .add_render_feature_plugin(debug_pip_renderer_plugin)
        .add_render_feature_plugin(text_renderer_plugin)
        .allow_use_render_thread(allow_use_render_thread);

    #[cfg(feature = "basic-pipeline")]
    {
        renderer_builder =
            renderer_builder.add_asset_plugin(Arc::new(MeshBasicAssetTypeRendererPlugin));
    }

    #[cfg(not(feature = "basic-pipeline"))]
    {
        renderer_builder =
            renderer_builder.add_asset_plugin(Arc::new(MeshAdvAssetTypeRendererPlugin));
    }

    #[cfg(feature = "egui")]
    {
        renderer_builder = renderer_builder.add_render_feature_plugin(egui_renderer_plugin)
    }

    let mut renderer_builder_result = {
        let extract_resources = ExtractResources::default();

        #[cfg(feature = "basic-pipeline")]
        let pipeline_plugin = Arc::new(BasicPipelineRendererPlugin);
        #[cfg(not(feature = "basic-pipeline"))]
        let pipeline_plugin = Arc::new(ModernPipelineRendererPlugin);

        renderer_builder.build(
            extract_resources,
            &rafx_api,
            asset_source,
            pipeline_plugin,
            || {
                None
                // Some(Box::new(DemoRendererThreadPool::new()))
            },
        )
    }?;

    let swapchain_def = RafxSwapchainDef {
        height: window_height,
        width: window_width,
        enable_vsync: true,
        color_space_priority: vec![RafxSwapchainColorSpace::Srgb],
    };

    let swapchain_helper = SwapchainHandler::create_swapchain(
        &mut renderer_builder_result.asset_manager,
        &mut renderer_builder_result.renderer,
        display,
        window,
        &swapchain_def,
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

        #[cfg(feature = "basic-pipeline")]
        MeshBasicRendererPlugin::legion_destroy(resources);
        #[cfg(not(feature = "basic-pipeline"))]
        MeshAdvRendererPlugin::legion_destroy(resources);
        SpriteRendererPlugin::legion_destroy(resources);
        SkyboxRendererPlugin::legion_destroy(resources);
        TileLayerRendererPlugin::legion_destroy(resources);
        Debug3DRendererPlugin::legion_destroy(resources);
        DebugPipRendererPlugin::legion_destroy(resources);
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
