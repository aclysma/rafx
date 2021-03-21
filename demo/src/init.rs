use crate::assets::font::FontAssetTypeRendererPlugin;
use crate::assets::gltf::GltfAssetTypeRendererPlugin;
use crate::features::debug3d::{Debug3DRendererPlugin, DebugDraw3DResource};
use crate::features::mesh::{MeshRenderNodeSet, MeshRendererPlugin};
use crate::features::sprite::{SpriteRenderNodeSet, SpriteRendererPlugin};
use crate::features::text::{TextRendererPlugin, TextResource};
use crate::render_graph_generator::DemoRenderGraphGenerator;
use crate::DemoRendererPlugin;
use legion::Resources;
use rafx::api::{RafxApi, RafxDeviceContext, RafxResult, RafxSwapchainHelper};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::nodes::{ExtractResources, RenderRegistry};
use rafx::renderer::ViewportsResource;
use rafx::renderer::{AssetSource, Renderer, RendererBuilder, SwapchainHandler};
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};

pub struct Sdl2Systems {
    pub context: sdl2::Sdl,
    pub video_subsystem: sdl2::VideoSubsystem,
    pub window: sdl2::video::Window,
}

pub fn sdl2_init() -> Sdl2Systems {
    // Setup SDL
    let context = sdl2::init().expect("Failed to initialize sdl2");
    let video_subsystem = context
        .video()
        .expect("Failed to create sdl video subsystem");

    // Create the window
    let window = video_subsystem
        .window("Rafx Demo", 900, 600)
        .position_centered()
        .allow_highdpi()
        .resizable()
        .build()
        .expect("Failed to create window");

    Sdl2Systems {
        context,
        video_subsystem,
        window,
    }
}

pub fn rendering_init(
    resources: &mut Resources,
    sdl2_window: &sdl2::video::Window,
    asset_source: AssetSource,
) -> RafxResult<()> {
    resources.insert(SpriteRenderNodeSet::default());
    resources.insert(MeshRenderNodeSet::default());
    resources.insert(StaticVisibilityNodeSet::default());
    resources.insert(DynamicVisibilityNodeSet::default());
    resources.insert(DebugDraw3DResource::new());
    resources.insert(TextResource::new());
    resources.insert(ViewportsResource::default());

    let rafx_api = rafx::api::RafxApi::new(sdl2_window, &Default::default())?;

    let mut renderer_builder = RendererBuilder::default();
    renderer_builder = renderer_builder
        .add_plugin(Box::new(FontAssetTypeRendererPlugin))
        .add_plugin(Box::new(GltfAssetTypeRendererPlugin))
        .add_plugin(Box::new(Debug3DRendererPlugin))
        .add_plugin(Box::new(TextRendererPlugin))
        .add_plugin(Box::new(SpriteRendererPlugin))
        .add_plugin(Box::new(MeshRendererPlugin))
        .add_plugin(Box::new(DemoRendererPlugin));

    #[cfg(feature = "use-imgui")]
    {
        use crate::features::imgui::ImguiRendererPlugin;
        let imgui_manager = crate::features::imgui::init_sdl2_imgui_manager(sdl2_window);
        resources.insert(imgui_manager);
        renderer_builder = renderer_builder.add_plugin(Box::new(ImguiRendererPlugin::default()));
    }

    let mut renderer_builder_result = {
        let mut extract_resources = ExtractResources::default();

        #[cfg(feature = "use-imgui")]
        let mut imgui_manager = resources
            .get_mut::<crate::features::imgui::Sdl2ImguiManager>()
            .unwrap();
        #[cfg(feature = "use-imgui")]
        extract_resources.insert(&mut *imgui_manager);

        let render_graph_generator = Box::new(DemoRenderGraphGenerator);

        renderer_builder.build(
            extract_resources,
            &rafx_api,
            asset_source,
            render_graph_generator,
        )
    }?;

    let (width, height) = sdl2_window.vulkan_drawable_size();
    let swapchain_helper = SwapchainHandler::create_swapchain(
        &mut renderer_builder_result.asset_manager,
        &mut renderer_builder_result.renderer,
        sdl2_window,
        width,
        height,
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

    Ok(())
}

pub fn rendering_destroy(resources: &mut Resources) -> RafxResult<()> {
    // Destroy these first
    {
        {
            let swapchain_helper = resources.remove::<RafxSwapchainHelper>().unwrap();
            let mut asset_manager = resources.get_mut::<AssetManager>().unwrap();
            let game_renderer = resources.get::<Renderer>().unwrap();
            SwapchainHandler::destroy_swapchain(
                swapchain_helper,
                &mut *asset_manager,
                &*game_renderer,
            )?;
        }

        resources.remove::<Renderer>();
        resources.remove::<SpriteRenderNodeSet>();
        resources.remove::<MeshRenderNodeSet>();
        resources.remove::<StaticVisibilityNodeSet>();
        resources.remove::<DynamicVisibilityNodeSet>();
        resources.remove::<DebugDraw3DResource>();
        resources.remove::<TextResource>();
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
