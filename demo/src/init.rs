use crate::assets::gltf::{GltfMaterialAsset, MeshAssetData};
use crate::features::debug3d::{Debug3dRenderFeature, DebugDraw3DResource};
use crate::features::text::{TextRenderFeature, TextResource};
#[cfg(feature = "use-imgui")]
use crate::features::imgui::ImGuiRenderFeature;
use crate::features::mesh::{MeshRenderFeature, MeshRenderNodeSet};
use crate::features::sprite::{SpriteRenderFeature, SpriteRenderNodeSet};
use crate::game_asset_lookup::MeshAsset;
use crate::game_asset_manager::GameAssetManager;
use crate::game_renderer::{GameRenderer, SwapchainHandler};
use crate::phases::PostProcessRenderPhase;
use crate::phases::TransparentRenderPhase;
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase, UiRenderPhase};
use distill::loader::{
    packfile_io::PackfileReader, storage::DefaultIndirectionResolver, Loader, RpcIO,
};
use legion::Resources;
use rafx::api::{RafxApi, RafxDeviceContext, RafxQueueType, RafxResult};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::distill_impl::ResourceAssetLoader;
use rafx::assets::{AssetManager, ComputePipelineAsset, ComputePipelineAssetData};
use rafx::assets::{BufferAsset, ImageAsset, MaterialAsset, MaterialInstanceAsset, ShaderAsset};
use rafx::assets::{
    BufferAssetData, ImageAssetData, MaterialAssetData, MaterialInstanceAssetData, ShaderAssetData,
};
use rafx::nodes::RenderRegistry;
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use crate::assets::font::{FontAssetData, FontAsset};

pub fn init_distill_daemon(
    resources: &mut Resources,
    connect_string: String,
) {
    let rpc_loader = RpcIO::new(connect_string).unwrap();
    let loader = Loader::new(Box::new(rpc_loader));
    let resolver = Box::new(DefaultIndirectionResolver);
    resources.insert(AssetResource::new(loader, resolver));
}

pub fn init_distill_packfile(
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

// Should occur *before* the renderer starts
#[cfg(feature = "use-imgui")]
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
) -> RafxResult<()> {
    resources.insert(SpriteRenderNodeSet::default());
    resources.insert(MeshRenderNodeSet::default());
    resources.insert(StaticVisibilityNodeSet::default());
    resources.insert(DynamicVisibilityNodeSet::default());
    resources.insert(DebugDraw3DResource::new());
    resources.insert(TextResource::new());

    #[allow(unused_mut)]
    let mut render_registry = rafx::nodes::RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<MeshRenderFeature>()
        .register_feature::<Debug3dRenderFeature>()
        .register_feature::<TextRenderFeature>()
        .register_render_phase::<OpaqueRenderPhase>("Opaque")
        .register_render_phase::<ShadowMapRenderPhase>("ShadowMap")
        .register_render_phase::<TransparentRenderPhase>("Transparent")
        .register_render_phase::<PostProcessRenderPhase>("PostProcess")
        .register_render_phase::<UiRenderPhase>("Ui");

    #[cfg(feature = "use-imgui")]
    {
        render_registry = render_registry.register_feature::<ImGuiRenderFeature>();
    }

    let render_registry = render_registry.build();

    let rafx_api = rafx::api::RafxApi::new(sdl2_window, &Default::default())?;

    let device_context = rafx_api.device_context();

    let graphics_queue = device_context.create_queue(RafxQueueType::Graphics)?;
    let transfer_queue = device_context.create_queue(RafxQueueType::Transfer)?;

    let resource_manager = {
        let mut asset_resource = resources.get_mut::<AssetResource>().unwrap();

        let asset_manager = rafx::assets::AssetManager::new(
            &device_context,
            &render_registry,
            asset_resource.loader(),
            rafx::assets::UploadQueueConfig {
                max_concurrent_uploads: 4,
                max_new_uploads_in_single_frame: 4,
                max_bytes_per_upload: 64 * 1024 * 1024,
            },
            &graphics_queue,
            &transfer_queue,
        );
        let loaders = asset_manager.create_loaders();

        asset_resource.add_storage_with_loader::<ShaderAssetData, ShaderAsset, _>(Box::new(
            ResourceAssetLoader(loaders.shader_loader),
        ));
        asset_resource
            .add_storage_with_loader::<ComputePipelineAssetData, ComputePipelineAsset, _>(
                Box::new(ResourceAssetLoader(loaders.compute_pipeline_loader)),
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

    resources.insert(rafx_api);
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

        asset_resource.add_storage_with_loader::<FontAssetData, FontAsset, _>(Box::new(
            ResourceAssetLoader(game_resource_manager.create_font_loader()),
        ));
        game_resource_manager
    };

    resources.insert(game_resource_manager);

    let game_renderer = GameRenderer::new(&resources, &graphics_queue, &transfer_queue).unwrap();
    resources.insert(game_renderer);

    let (width, height) = sdl2_window.vulkan_drawable_size();
    SwapchainHandler::create_swapchain(resources, sdl2_window, width, height)?;

    Ok(())
}

pub fn rendering_destroy(resources: &mut Resources) -> RafxResult<()> {
    // Destroy these first
    {
        SwapchainHandler::destroy_swapchain(resources)?;
        resources.remove::<GameRenderer>();
        resources.remove::<SpriteRenderNodeSet>();
        resources.remove::<MeshRenderNodeSet>();
        resources.remove::<StaticVisibilityNodeSet>();
        resources.remove::<DynamicVisibilityNodeSet>();
        resources.remove::<DebugDraw3DResource>();
        resources.remove::<TextResource>();
        resources.remove::<GameAssetManager>();
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
