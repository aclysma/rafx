use crate::asset_resource::AssetResource;
use distill::loader::handle::Handle;
use distill::loader::storage::LoadStatus;
use rafx::api::RafxResult;
use rafx::assets::MaterialAsset;
use rafx::assets::{AssetManager, ComputePipelineAsset};

fn wait_for_asset_to_load<T>(
    asset_handle: &distill::loader::handle::Handle<T>,
    asset_resource: &mut AssetResource,
    asset_manager: &mut AssetManager,
    asset_name: &str,
) -> RafxResult<()> {
    loop {
        asset_resource.update();
        asset_manager.update_asset_loaders()?;
        match asset_resource.load_status(&asset_handle) {
            LoadStatus::NotRequested => {
                unreachable!();
            }
            LoadStatus::Unresolved => {
                log::info!(
                    "blocked waiting for asset to resolve {} {:?}",
                    asset_name,
                    asset_handle
                );
            }
            LoadStatus::Loading => {
                log::info!(
                    "blocked waiting for asset to load {} {:?}",
                    asset_name,
                    asset_handle
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
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
    pub compute_test: Handle<ComputePipelineAsset>,
}

impl GameRendererStaticResources {
    pub fn new(
        asset_resource: &mut AssetResource,
        asset_manager: &mut AssetManager,
    ) -> RafxResult<Self> {
        //
        // Sprite resources
        //
        let sprite_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("pipelines/sprite.material");

        //
        // Debug resources
        //
        let debug3d_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("pipelines/debug.material");

        //
        // Bloom extract resources
        //
        // let bloom_extract_material = asset_resource
        //     .load_asset_path::<MaterialAsset, _>("pipelines/bloom_extract.material");
        let bloom_extract_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("pipelines/bloom_extract.material");
        //.load_asset::<MaterialAsset>(asset_uuid!("4c5509e3-4a9f-45c2-a6dc-862a925d2341"));

        //
        // Bloom blur resources
        //
        let bloom_blur_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("pipelines/bloom_blur.material");

        //
        // Bloom combine resources
        //
        let bloom_combine_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("pipelines/bloom_combine.material");

        //
        // ImGui resources
        //
        let imgui_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("pipelines/imgui.material");

        //
        // Compute pipeline
        //
        let compute_test = asset_resource
            .load_asset_path::<ComputePipelineAsset, _>("pipelines/compute_test.compute");

        wait_for_asset_to_load(
            &sprite_material,
            asset_resource,
            asset_manager,
            "sprite_material",
        )?;

        wait_for_asset_to_load(
            &debug3d_material,
            asset_resource,
            asset_manager,
            "debug material",
        )?;

        wait_for_asset_to_load(
            &bloom_extract_material,
            asset_resource,
            asset_manager,
            "bloom extract material",
        )?;

        wait_for_asset_to_load(
            &bloom_blur_material,
            asset_resource,
            asset_manager,
            "bloom blur material",
        )?;

        wait_for_asset_to_load(
            &bloom_combine_material,
            asset_resource,
            asset_manager,
            "bloom combine material",
        )?;

        wait_for_asset_to_load(
            &imgui_material,
            asset_resource,
            asset_manager,
            "imgui material",
        )?;

        wait_for_asset_to_load(
            &compute_test,
            asset_resource,
            asset_manager,
            "compute pipeline",
        )?;

        Ok(GameRendererStaticResources {
            sprite_material,
            debug3d_material,
            bloom_extract_material,
            bloom_blur_material,
            bloom_combine_material,
            imgui_material,
            compute_test,
        })
    }
}
