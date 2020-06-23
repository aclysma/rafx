use renderer::assets::asset_resource::AssetResource;
use legion::prelude::Resources;
use renderer::vulkan::{
    LogicalSize, VkContextBuilder, MsaaLevel, VkDeviceContext, VkSurface, VkContext,
};
use crate::features::sprite::{SpriteRenderNodeSet, SpriteRenderFeature};
use crate::features::mesh::{MeshRenderNodeSet, MeshRenderFeature};
use renderer::visibility::{StaticVisibilityNodeSet, DynamicVisibilityNodeSet};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use crate::game_renderer::{SwapchainLifetimeListener, GameRenderer};
use renderer::features::renderpass::debug_renderpass::DebugDraw3DResource;
use renderer::nodes::RenderRegistry;
use crate::assets::gltf::{MeshAsset, GltfMaterialAsset};
use crate::resource_manager::GameResourceManager;
use renderer::resources::ResourceManager;

pub fn logging_init() {
    let mut log_level = log::LevelFilter::Info;
    //#[cfg(debug_assertions)]
    {
        log_level = log::LevelFilter::Debug;
    }

    // Setup logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_module(
            "renderer_resources::resource_managers::descriptor_sets",
            log::LevelFilter::Info,
        )
        .filter_module("renderer_base", log::LevelFilter::Info)
        .filter_level(log_level)
        // .format(|buf, record| { //TODO: Get a frame count in here
        //     writeln!(buf,
        //              "{} [{}] - {}",
        //              chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
        //              record.level(),
        //              record.args()
        //     )
        // })
        .init();
}

pub fn atelier_init(resources: &mut Resources) {
    resources.insert(AssetResource::default());
}

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

    // Default window size
    let logical_size = LogicalSize {
        width: 900,
        height: 600,
    };

    // Create the window
    let window = video_subsystem
        .window(
            "Renderer Prototype",
            logical_size.width,
            logical_size.height,
        )
        .position_centered()
        .allow_highdpi()
        .resizable()
        .vulkan()
        .build()
        .expect("Failed to create window");

    Sdl2Systems {
        context,
        video_subsystem,
        window,
    }
}

// Should occur *before* the renderer starts
pub fn imgui_init(
    resources: &mut Resources,
    sdl2_window: &sdl2::video::Window,
) {
    // Load imgui, we do it a little early because it wants to have the actual SDL2 window and
    // doesn't work with the thin window wrapper
    let imgui_manager = renderer::features::imgui_support::init_imgui_manager(sdl2_window);
    resources.insert(imgui_manager);
}

pub fn rendering_init(
    resources: &mut Resources,
    sdl2_window: &sdl2::video::Window,
) {
    // Thin window wrapper to decouple the renderer from a specific windowing crate
    let window_wrapper = Sdl2Window::new(&sdl2_window);

    resources.insert(SpriteRenderNodeSet::default());
    resources.insert(MeshRenderNodeSet::default());
    resources.insert(StaticVisibilityNodeSet::default());
    resources.insert(DynamicVisibilityNodeSet::default());

    let mut context = VkContextBuilder::new()
        .use_vulkan_debug_layer(false)
        .msaa_level_priority(vec![MsaaLevel::Sample4])
        //.msaa_level_priority(vec![MsaaLevel::Sample1])
        .prefer_mailbox_present_mode();

    //#[cfg(debug_assertions)]
    {
        context = context.use_vulkan_debug_layer(true);
    }

    let vk_context = context.build(&window_wrapper).unwrap();
    let device_context = vk_context.device_context().clone();
    let resource_manager = {
        let mut asset_resourceh = resources.get_mut::<AssetResource>().unwrap();
        renderer::resources::create_resource_manager(&device_context, &mut *asset_resourceh)
    };
    resources.insert(vk_context);
    resources.insert(device_context);
    resources.insert(resource_manager);



    {
        //
        // Create the game resource manager
        //
        let device_context = resources.get_mut::<VkDeviceContext>().unwrap().clone();
        let mut resource_manager = GameResourceManager::new(&device_context);
        resources.insert(resource_manager);

        let mut asset_resource_fetch = resources.get_mut::<AssetResource>().unwrap();
        let asset_resource = &mut *asset_resource_fetch;

        let mut resource_manager_fetch = resources.get_mut::<GameResourceManager>().unwrap();
        let resource_manager = &mut *resource_manager_fetch;

        asset_resource.add_storage_with_load_handler::<MeshAsset, _>(Box::new(
            resource_manager.create_mesh_load_handler(),
        ));

        asset_resource.add_storage::<GltfMaterialAsset>();
    }

    let mut render_registry_builder = renderer::features::create_default_registry_builder();
    let render_registry = render_registry_builder
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<MeshRenderFeature>()
        .build();
    resources.insert(render_registry);

    renderer::features::init_renderer_features(resources);

    let mut game_renderer = GameRenderer::new(&window_wrapper, &resources).unwrap();
    resources.insert(game_renderer);

    let window_surface =
        SwapchainLifetimeListener::create_surface(resources, &window_wrapper).unwrap();
    resources.insert(window_surface);
}

pub fn rendering_destroy(resources: &mut Resources) {
    // Destroy these first
    {
        SwapchainLifetimeListener::tear_down(resources);
        resources.remove::<VkSurface>();
        resources.remove::<GameRenderer>();
        resources.remove::<VkDeviceContext>();
        resources.remove::<SpriteRenderNodeSet>();
        resources.remove::<MeshRenderNodeSet>();
        resources.remove::<StaticVisibilityNodeSet>();
        resources.remove::<DynamicVisibilityNodeSet>();

        renderer::features::destroy_renderer_features(resources);

        resources.remove::<GameResourceManager>();

        resources.remove::<RenderRegistry>();
        resources.remove::<ResourceManager>();
    }

    // Drop this one last
    resources.remove::<VkContext>();
}
