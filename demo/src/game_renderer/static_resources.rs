use distill::loader::handle::Handle;
use distill::loader::storage::LoadStatus;
use rafx::api::RafxResult;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::{AssetManager, ComputePipelineAsset};
use rafx::assets::{ImageAsset, MaterialAsset};
use crate::assets::font::FontAsset;
use crate::game_asset_manager::GameAssetManager;

fn wait_for_asset_to_load<T>(
    asset_handle: &distill::loader::handle::Handle<T>,
    asset_resource: &mut AssetResource,
    asset_manager: &mut AssetManager,
    game_asset_manager: &mut GameAssetManager,
    asset_name: &str,
) -> RafxResult<()> {
    const PRINT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1000);
    let mut last_print_time = None;

    fn on_interval<F: Fn()>(
        interval: std::time::Duration,
        last_time: &mut Option<std::time::Instant>,
        f: F,
    ) {
        let now = std::time::Instant::now();

        if last_time.is_none() || now - last_time.unwrap() >= interval {
            (f)();
            *last_time = Some(now);
        }
    }

    loop {
        asset_resource.update();
        asset_manager.update_asset_loaders()?;
        game_asset_manager.update_asset_loaders(asset_manager)?;
        match asset_resource.load_status(&asset_handle) {
            LoadStatus::NotRequested => {
                unreachable!();
            }
            LoadStatus::Unresolved => {
                on_interval(PRINT_INTERVAL, &mut last_print_time, || {
                    log::info!(
                        "blocked waiting for asset to resolve {} {:?}",
                        asset_name,
                        asset_handle
                    );
                });
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            LoadStatus::Loading => {
                on_interval(PRINT_INTERVAL, &mut last_print_time, || {
                    log::info!(
                        "blocked waiting for asset to load {} {:?}",
                        asset_name,
                        asset_handle
                    );
                });
                std::thread::sleep(std::time::Duration::from_millis(1));
                // keep waiting
            }
            LoadStatus::Loaded => {
                break Ok(());
            }
            LoadStatus::Unloading => unreachable!(),
            LoadStatus::DoesNotExist => {
                println!("Essential asset not found");
            }
            LoadStatus::Error(err) => {
                println!("Error loading essential asset {:?}", err);
            }
        }
    }
}

#[derive(Clone)]
pub struct GameRendererStaticResources {
    pub sprite_material: Handle<MaterialAsset>,
    pub debug3d_material: Handle<MaterialAsset>,
    pub bloom_extract_material: Handle<MaterialAsset>,
    pub bloom_blur_material: Handle<MaterialAsset>,
    pub bloom_combine_material: Handle<MaterialAsset>,
    pub imgui_material: Handle<MaterialAsset>,
    pub skybox_material: Handle<MaterialAsset>,
    pub skybox_texture: Handle<ImageAsset>,
    pub compute_test: Handle<ComputePipelineAsset>,
    pub text_material: Handle<MaterialAsset>,
    pub default_font: Handle<FontAsset>,
}

impl GameRendererStaticResources {
    pub fn new(
        asset_resource: &mut AssetResource,
        asset_manager: &mut AssetManager,
        game_asset_manager: &mut GameAssetManager,
    ) -> RafxResult<Self> {
        //
        // Sprite resources
        //
        let sprite_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/sprite.material");

        //
        // Debug resources
        //
        let debug3d_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/debug.material");

        //
        // Bloom extract resources
        //
        // let bloom_extract_material = asset_resource
        //     .load_asset_path::<MaterialAsset, _>("pipelines/bloom_extract.material");
        let bloom_extract_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/bloom_extract.material");
        //.load_asset::<MaterialAsset>(asset_uuid!("4c5509e3-4a9f-45c2-a6dc-862a925d2341"));

        //
        // Bloom blur resources
        //
        let bloom_blur_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/bloom_blur.material");

        //
        // Bloom combine resources
        //
        let bloom_combine_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/bloom_combine.material");

        //
        // ImGui resources
        //
        let imgui_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/imgui.material");

        //
        // Skybox resources
        //
        let skybox_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/skybox.material");
        let skybox_texture =
            asset_resource.load_asset_path::<ImageAsset, _>("textures/skybox.basis");

        //
        // Compute pipeline
        //
        let compute_test = asset_resource
            .load_asset_path::<ComputePipelineAsset, _>("compute_pipelines/compute_test.compute");

        //
        // Text rendering resources
        //
        let text_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/text.material");
        let default_font = asset_resource
            .load_asset_path::<FontAsset, _>("fonts/mplus-1p-regular.ttf");

        wait_for_asset_to_load(
            &sprite_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "sprite_material",
        )?;

        wait_for_asset_to_load(
            &debug3d_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "debug material",
        )?;

        wait_for_asset_to_load(
            &bloom_extract_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "bloom extract material",
        )?;

        wait_for_asset_to_load(
            &bloom_blur_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "bloom blur material",
        )?;

        wait_for_asset_to_load(
            &bloom_combine_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "bloom combine material",
        )?;

        wait_for_asset_to_load(
            &imgui_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "imgui material",
        )?;

        wait_for_asset_to_load(
            &skybox_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "skybox material",
        )?;

        wait_for_asset_to_load(
            &skybox_texture,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "skybox texture",
        )?;

        wait_for_asset_to_load(
            &compute_test,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "compute pipeline",
        )?;

        wait_for_asset_to_load(
            &text_material,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "text material",
        )?;

        wait_for_asset_to_load(
            &default_font,
            asset_resource,
            asset_manager,
            game_asset_manager,
            "default font",
        )?;

        Ok(GameRendererStaticResources {
            sprite_material,
            debug3d_material,
            bloom_extract_material,
            bloom_blur_material,
            bloom_combine_material,
            imgui_material,
            skybox_material,
            skybox_texture,
            compute_test,
            text_material,
            default_font,
        })
    }
}
