use crate::asset_resource::AssetResource;
use legion::prelude::Resources;
use renderer::vulkan::{
    LogicalSize, VkContextBuilder, MsaaLevel, VkDeviceContext, VkSurface, VkContext,
};
use crate::features::sprite::{SpriteRenderNodeSet, SpriteRenderFeature};
use crate::features::mesh::{MeshRenderNodeSet, MeshRenderFeature};
use renderer::visibility::{StaticVisibilityNodeSet, DynamicVisibilityNodeSet};
use renderer_shell_vulkan_sdl2::Sdl2Window;
use crate::game_renderer::{SwapchainLifetimeListener, GameRenderer};
use crate::features::debug3d::{DebugDraw3DResource, Debug3dRenderFeature};
use renderer::nodes::RenderRegistry;
use crate::assets::gltf::{MeshAssetData, GltfMaterialAsset};
use crate::resource_manager::GameResourceManager;
use renderer::resources::ResourceManager;
use crate::phases::{OpaqueRenderPhase, UiRenderPhase};
use crate::phases::TransparentRenderPhase;
use crate::features::imgui::ImGuiRenderFeature;
use crate::game_asset_lookup::MeshAsset;
use renderer::resources::{ShaderAsset, PipelineAsset, RenderpassAsset, MaterialAsset, MaterialInstanceAsset, ImageAsset, BufferAsset};
use renderer::assets::{ShaderAssetData, PipelineAssetData, RenderpassAssetData, MaterialAssetData, MaterialInstanceAssetData, ImageAssetData, BufferAssetData};
use crate::asset_loader::ResourceAssetLoader;

pub fn logging_init() {
    #[allow(unused_assignments)]
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
        .filter_module("renderer_shell_vulkan::device", log::LevelFilter::Debug)
        .filter_module("renderer_nodes", log::LevelFilter::Info)
        .filter_module("renderer_visibility", log::LevelFilter::Info)
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
    let imgui_manager = crate::imgui_support::init_imgui_manager(sdl2_window);
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
    resources.insert(DebugDraw3DResource::new());

    let mut context = VkContextBuilder::new()
        .use_vulkan_debug_layer(false)
        .msaa_level_priority(vec![MsaaLevel::Sample4])
        //.msaa_level_priority(vec![MsaaLevel::Sample1])
        .prefer_mailbox_present_mode();

    #[cfg(debug_assertions)]
    {
        context = context.use_vulkan_debug_layer(true);
    }

    let vk_context = context.build(&window_wrapper).unwrap();
    let device_context = vk_context.device_context().clone();
    let resource_manager = renderer::resources::ResourceManager::new(&device_context);

    {
        let loaders = resource_manager.create_loaders();
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        asset_resource.add_storage_with_loader::<ShaderAssetData, ShaderAsset, _>(
            Box::new(ResourceAssetLoader(loaders.shader_loader)),
        );
        asset_resource.add_storage_with_loader::<PipelineAssetData, PipelineAsset, _>(
            Box::new(ResourceAssetLoader(loaders.pipeline_loader)),
        );
        asset_resource.add_storage_with_loader::<RenderpassAssetData, RenderpassAsset, _>(
            Box::new(ResourceAssetLoader(loaders.renderpass_loader)),
        );
        asset_resource.add_storage_with_loader::<MaterialAssetData, MaterialAsset, _>(
            Box::new(ResourceAssetLoader(loaders.material_loader)),
        );
        asset_resource.add_storage_with_loader::<MaterialInstanceAssetData, MaterialInstanceAsset, _>(
            Box::new(ResourceAssetLoader(loaders.material_instance_loader)),
        );
        asset_resource.add_storage_with_loader::<ImageAssetData, ImageAsset, _>(
            Box::new(ResourceAssetLoader(loaders.image_loader)),
        );
        asset_resource.add_storage_with_loader::<BufferAssetData, BufferAsset, _>(
            Box::new(ResourceAssetLoader(loaders.buffer_loader)),
        );
    }

    resources.insert(vk_context);
    resources.insert(device_context);
    resources.insert(resource_manager);

    {
        //
        // Create the game resource manager
        //
        let resource_manager = GameResourceManager::new();
        resources.insert(resource_manager);

        let mut asset_resource_fetch = resources.get_mut::<AssetResource>().unwrap();
        let asset_resource = &mut *asset_resource_fetch;

        let mut resource_manager_fetch = resources.get_mut::<GameResourceManager>().unwrap();
        let resource_manager = &mut *resource_manager_fetch;

        asset_resource.add_storage_with_loader::<MeshAssetData, MeshAsset, _>(
            Box::new(ResourceAssetLoader(resource_manager.create_mesh_loader()))
        );

        asset_resource.add_storage::<GltfMaterialAsset>();
    }

    let render_registry = renderer::nodes::RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<MeshRenderFeature>()
        .register_feature::<Debug3dRenderFeature>()
        .register_feature::<ImGuiRenderFeature>()
        .register_render_phase::<OpaqueRenderPhase>()
        .register_render_phase::<TransparentRenderPhase>()
        .register_render_phase::<UiRenderPhase>()
        .build();
    resources.insert(render_registry);

    let game_renderer = GameRenderer::new(&window_wrapper, &resources).unwrap();
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
        resources.remove::<DebugDraw3DResource>();
        resources.remove::<GameResourceManager>();
        resources.remove::<RenderRegistry>();

        // Remove the asset resource because we have asset storages that reference resources
        resources.remove::<AssetResource>();

        resources.remove::<ResourceManager>();
    }

    // Drop this one last
    resources.remove::<VkContext>();
}
