use crate::asset_loader::ResourceAssetLoader;
use crate::asset_resource::AssetResource;
use crate::assets::gltf::{GltfMaterialAsset, MeshAssetData};
use crate::features::debug3d::{Debug3dRenderFeature, DebugDraw3DResource};
use crate::features::imgui::ImGuiRenderFeature;
use crate::features::mesh::{MeshRenderFeature, MeshRenderNodeSet};
use crate::features::sprite::{SpriteRenderFeature, SpriteRenderNodeSet};
use crate::game_asset_lookup::MeshAsset;
use crate::game_asset_manager::GameAssetManager;
use crate::game_renderer::{GameRenderer, SwapchainLifetimeListener};
use crate::phases::TransparentRenderPhase;
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase, UiRenderPhase};
use atelier_assets::loader::{
    packfile_io::PackfileReader, storage::DefaultIndirectionResolver, Loader, RpcIO,
};
use legion::Resources;
use renderer::assets::AssetManager;
use renderer::assets::{
    BufferAsset, ImageAsset, MaterialAsset, MaterialInstanceAsset, PipelineAsset, RenderpassAsset,
    ShaderAsset,
};
use renderer::assets::{
    BufferAssetData, ImageAssetData, MaterialAssetData, MaterialInstanceAssetData,
    PipelineAssetData, RenderpassAssetData, ShaderAssetData,
};
use renderer::nodes::RenderRegistry;
use renderer::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use renderer::vulkan::{
    LogicalSize, MsaaLevel, VkContext, VkContextBuilder, VkDeviceContext, VkSurface,
    VulkanLinkMethod,
};
use renderer_shell_vulkan_sdl2::Sdl2Window;

pub fn atelier_init_daemon(
    resources: &mut Resources,
    connect_string: String,
) {
    let rpc_loader = RpcIO::new(connect_string).unwrap();
    let loader = Loader::new(Box::new(rpc_loader));
    let resolver = Box::new(DefaultIndirectionResolver);
    resources.insert(AssetResource::new(loader, resolver));
}

pub fn atelier_init_packfile(
    resources: &mut Resources,
    pack_file: &std::path::Path,
) {
    let packfile = std::fs::File::open(pack_file).unwrap();
    let packfile_loader = PackfileReader::new(packfile).unwrap();
    let loader = Loader::new(Box::new(packfile_loader));
    let resolver = Box::new(DefaultIndirectionResolver);
    resources.insert(AssetResource::new(loader, resolver));
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

    #[cfg(debug_assertions)]
    let use_vulkan_debug_layer = true;
    #[cfg(not(debug_assertions))]
    let use_vulkan_debug_layer = false;

    #[cfg(not(feature = "static-vulkan"))]
    let link_method = VulkanLinkMethod::Dynamic;
    #[cfg(feature = "static-vulkan")]
    let link_method = VulkanLinkMethod::Static;

    let context = VkContextBuilder::new()
        .link_method(link_method)
        .use_vulkan_debug_layer(use_vulkan_debug_layer)
        .msaa_level_priority(vec![MsaaLevel::Sample4])
        //.msaa_level_priority(vec![MsaaLevel::Sample1])
        .prefer_mailbox_present_mode();

    let render_registry = renderer::nodes::RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<MeshRenderFeature>()
        .register_feature::<Debug3dRenderFeature>()
        .register_feature::<ImGuiRenderFeature>()
        .register_render_phase::<OpaqueRenderPhase>("Opaque")
        .register_render_phase::<ShadowMapRenderPhase>("ShadowMap")
        .register_render_phase::<TransparentRenderPhase>("Transparent")
        .register_render_phase::<UiRenderPhase>("Ui")
        .build();

    let vk_context = context.build(&window_wrapper).unwrap();
    let device_context = vk_context.device_context().clone();

    let resource_manager = {
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        let asset_manager = renderer::assets::AssetManager::new(
            &device_context,
            &render_registry,
            asset_resource.loader(),
        );
        let loaders = asset_manager.create_loaders();

        asset_resource.add_storage_with_loader::<ShaderAssetData, ShaderAsset, _>(Box::new(
            ResourceAssetLoader(loaders.shader_loader),
        ));
        asset_resource.add_storage_with_loader::<PipelineAssetData, PipelineAsset, _>(Box::new(
            ResourceAssetLoader(loaders.pipeline_loader),
        ));
        asset_resource.add_storage_with_loader::<RenderpassAssetData, RenderpassAsset, _>(
            Box::new(ResourceAssetLoader(loaders.renderpass_loader)),
        );
        asset_resource.add_storage_with_loader::<MaterialAssetData, MaterialAsset, _>(Box::new(
            ResourceAssetLoader(loaders.material_loader),
        ));
        asset_resource
            .add_storage_with_loader::<MaterialInstanceAssetData, MaterialInstanceAsset, _>(
                Box::new(ResourceAssetLoader(loaders.material_instance_loader)),
            );
        asset_resource.add_storage_with_loader::<ImageAssetData, ImageAsset, _>(Box::new(
            ResourceAssetLoader(loaders.image_loader),
        ));
        asset_resource.add_storage_with_loader::<BufferAssetData, BufferAsset, _>(Box::new(
            ResourceAssetLoader(loaders.buffer_loader),
        ));

        asset_manager
    };

    resources.insert(vk_context);
    resources.insert(device_context);
    resources.insert(resource_manager);
    resources.insert(render_registry);

    let game_resource_manager = {
        //
        // Create the game resource manager
        //
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        let game_resource_manager = GameAssetManager::new(asset_resource.loader());

        asset_resource.add_storage_with_loader::<MeshAssetData, MeshAsset, _>(Box::new(
            ResourceAssetLoader(game_resource_manager.create_mesh_loader()),
        ));

        asset_resource.add_storage::<GltfMaterialAsset>();
        game_resource_manager
    };

    resources.insert(game_resource_manager);

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
        resources.remove::<GameAssetManager>();
        resources.remove::<RenderRegistry>();

        // Remove the asset resource because we have asset storages that reference resources
        resources.remove::<AssetResource>();

        resources.remove::<AssetManager>();
    }

    // Drop this one last
    resources.remove::<VkContext>();
}
