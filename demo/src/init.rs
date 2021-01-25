use crate::asset_loader::ResourceAssetLoader;
use crate::asset_resource::AssetResource;
use crate::assets::gltf::{GltfMaterialAsset, MeshAssetData};
use crate::features::debug3d::{Debug3dRenderFeature, DebugDraw3DResource};
use crate::features::imgui::ImGuiRenderFeature;
use crate::features::mesh::{MeshRenderFeature, MeshRenderNodeSet};
use crate::features::sprite::{SpriteRenderFeature, SpriteRenderNodeSet};
use crate::game_asset_lookup::MeshAsset;
use crate::game_asset_manager::GameAssetManager;
use crate::game_renderer::{GameRenderer, SwapchainHandler};
use crate::phases::PostProcessRenderPhase;
use crate::phases::TransparentRenderPhase;
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase, UiRenderPhase};
use atelier_assets::loader::{
    packfile_io::PackfileReader, storage::DefaultIndirectionResolver, Loader, RpcIO,
};
use legion::Resources;
use rafx::api::{RafxApi, RafxDeviceContext, RafxQueueType, RafxResult};
use rafx::assets::{AssetManager, ComputePipelineAsset, ComputePipelineAssetData};
use rafx::assets::{
    BufferAsset, GraphicsPipelineAsset, ImageAsset, MaterialAsset, MaterialInstanceAsset,
    ShaderAsset,
};
use rafx::assets::{
    BufferAssetData, GraphicsPipelineAssetData, ImageAssetData, MaterialAssetData,
    MaterialInstanceAssetData, ShaderAssetData,
};
use rafx::nodes::RenderRegistry;
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};

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

    let render_registry = rafx::nodes::RenderRegistryBuilder::default()
        .register_feature::<SpriteRenderFeature>()
        .register_feature::<MeshRenderFeature>()
        .register_feature::<Debug3dRenderFeature>()
        .register_feature::<ImGuiRenderFeature>()
        .register_render_phase::<OpaqueRenderPhase>("Opaque")
        .register_render_phase::<ShadowMapRenderPhase>("ShadowMap")
        .register_render_phase::<TransparentRenderPhase>("Transparent")
        .register_render_phase::<PostProcessRenderPhase>("PostProcess")
        .register_render_phase::<UiRenderPhase>("Ui")
        .build();

    #[cfg(feature = "rafx-vulkan")]
    let rafx_api = {
        use rafx::api::vulkan::VulkanLinkMethod;

        #[cfg(debug_assertions)]
        let validation_mode = rafx::api::RafxValidationMode::EnabledIfAvailable;
        #[cfg(not(debug_assertions))]
        let validation_mode = rafx::api::RafxValidationMode::Disabled;

        #[cfg(not(feature = "static-vulkan"))]
        let link_method = VulkanLinkMethod::Dynamic;
        #[cfg(feature = "static-vulkan")]
        let link_method = VulkanLinkMethod::Static;

        rafx::api::RafxApi::new_vulkan(
            sdl2_window,
            &rafx::api::RafxApiDef { validation_mode },
            &rafx::api::RafxApiDefVulkan {
                link_method: Some(link_method),
                app_name: None,
            },
        )
    }?;

    #[cfg(feature = "rafx-metal")]
    let rafx_api = {
        #[cfg(debug_assertions)]
        let validation_mode = rafx::api::RafxValidationMode::EnabledIfAvailable;
        #[cfg(not(debug_assertions))]
        let validation_mode = rafx::api::RafxValidationMode::Disabled;

        rafx::api::RafxApi::new_metal(
            sdl2_window,
            &rafx::api::RafxApiDef { validation_mode },
            &rafx::api::RafxApiDefMetal {},
        )
    }?;

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
                max_bytes_per_upload: 32 * 1024 * 1024,
            },
            &graphics_queue,
            &transfer_queue,
        );
        let loaders = asset_manager.create_loaders();

        asset_resource.add_storage_with_loader::<ShaderAssetData, ShaderAsset, _>(Box::new(
            ResourceAssetLoader(loaders.shader_loader),
        ));
        asset_resource
            .add_storage_with_loader::<GraphicsPipelineAssetData, GraphicsPipelineAsset, _>(
                Box::new(ResourceAssetLoader(loaders.graphics_pipeline_loader)),
            );
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
