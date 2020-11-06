use crate::asset_resource::AssetResource;
use renderer::assets::resources::ResourceManager;
use atelier_assets::loader::handle::Handle;
use atelier_assets::core::asset_uuid;
use atelier_assets::loader::storage::LoadStatus;
use atelier_assets::core as atelier_core;
use ash::prelude::VkResult;
use renderer::assets::MaterialAsset;

fn wait_for_asset_to_load<T>(
    asset_handle: &atelier_assets::loader::handle::Handle<T>,
    asset_resource: &mut AssetResource,
    resource_manager: &mut ResourceManager,
    asset_name: &str,
) -> VkResult<()> {
    loop {
        asset_resource.update();
        resource_manager.update_resources()?;
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

pub struct GameRendererStaticResources {
    pub sprite_material: Handle<MaterialAsset>,
    pub debug3d_material: Handle<MaterialAsset>,
    pub bloom_extract_material: Handle<MaterialAsset>,
    pub bloom_blur_material: Handle<MaterialAsset>,
    pub bloom_combine_material: Handle<MaterialAsset>,
    pub imgui_material: Handle<MaterialAsset>,
}

impl GameRendererStaticResources {
    pub fn new(
        asset_resource: &mut AssetResource,
        resource_manager: &mut ResourceManager,
    ) -> VkResult<Self> {
        //
        // Sprite resources
        //
        let sprite_material = asset_resource
            .load_asset::<MaterialAsset>(asset_uuid!("f8c4897e-7c1d-4736-93b7-f2deda158ec7"));

        //
        // Debug resources
        //
        let debug3d_material = asset_resource
            .load_asset::<MaterialAsset>(asset_uuid!("11d3b144-f564-42c9-b31f-82c8a938bf85"));

        //
        // Bloom extract resources
        //
        let bloom_extract_material = asset_resource
            .load_asset::<MaterialAsset>(asset_uuid!("822c8e08-2720-4002-81da-fd9c4d61abdd"));

        //
        // Bloom blur resources
        //
        let bloom_blur_material = asset_resource
            .load_asset::<MaterialAsset>(asset_uuid!("22aae4c1-fd0f-414a-9de1-7f68bdf1bfb1"));

        //
        // Bloom combine resources
        //
        let bloom_combine_material = asset_resource
            .load_asset::<MaterialAsset>(asset_uuid!("256e6a2d-669b-426b-900d-3bcc4249a063"));

        //
        // ImGui resources
        //
        let imgui_material = asset_resource
            .load_asset::<MaterialAsset>(asset_uuid!("b1cd2431-5cf8-4e9c-b7f0-569ba74e0981"));

        wait_for_asset_to_load(
            &sprite_material,
            asset_resource,
            resource_manager,
            "sprite_material",
        )?;

        wait_for_asset_to_load(
            &debug3d_material,
            asset_resource,
            resource_manager,
            "debub material",
        )?;

        wait_for_asset_to_load(
            &bloom_extract_material,
            asset_resource,
            resource_manager,
            "bloom extract material",
        )?;

        wait_for_asset_to_load(
            &bloom_blur_material,
            asset_resource,
            resource_manager,
            "bloom blur material",
        )?;

        wait_for_asset_to_load(
            &bloom_combine_material,
            asset_resource,
            resource_manager,
            "bloom combine material",
        )?;

        wait_for_asset_to_load(
            &imgui_material,
            asset_resource,
            resource_manager,
            "imgui material",
        )?;

        Ok(GameRendererStaticResources {
            sprite_material,
            debug3d_material,
            bloom_extract_material,
            bloom_blur_material,
            bloom_combine_material,
            imgui_material,
        })
    }
}
