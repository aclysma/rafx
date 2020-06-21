use renderer_assets::asset_resource::AssetResource;
use renderer_resources::resource_managers::ResourceManager;
use renderer_assets::pipeline::pipeline::MaterialAsset;
use ash::vk;
use atelier_assets::loader::handle::Handle;
use atelier_assets::core::asset_uuid;
use atelier_assets::core::AssetUuid;
use atelier_assets::loader::LoadStatus;
use atelier_assets::core as atelier_core;
use ash::prelude::VkResult;
use atelier_assets::loader::handle::AssetHandle;

fn begin_load_asset<T>(
    asset_uuid: AssetUuid,
    asset_resource: &AssetResource,
) -> atelier_assets::loader::handle::Handle<T> {
    use atelier_assets::loader::Loader;
    let load_handle = asset_resource.loader().add_ref(asset_uuid);
    atelier_assets::loader::handle::Handle::<T>::new(asset_resource.tx().clone(), load_handle)
}

fn wait_for_asset_to_load<T>(
    asset_handle: &atelier_assets::loader::handle::Handle<T>,
    asset_resource: &mut AssetResource,
    resource_manager: &mut ResourceManager,
    asset_name: &str
) {
    loop {
        asset_resource.update();
        resource_manager.update_resources();
        match asset_handle.load_status(asset_resource.loader()) {
            LoadStatus::NotRequested => {
                unreachable!();
            }
            LoadStatus::Loading => {
                log::info!("blocked waiting for asset to load {} {:?}", asset_name, asset_handle);
                std::thread::sleep(std::time::Duration::from_millis(10));
                // keep waiting
            }
            LoadStatus::Loaded => {
                break;
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
    pub debug_material: Handle<MaterialAsset>,
    pub mesh_material: Handle<MaterialAsset>,
    pub bloom_extract_material: Handle<MaterialAsset>,
    pub bloom_blur_material: Handle<MaterialAsset>,
    pub bloom_combine_material: Handle<MaterialAsset>,
}

impl GameRendererStaticResources {
    pub fn new(
        asset_resource: &mut AssetResource,
        resource_manager: &mut ResourceManager
    ) -> VkResult<Self> {
        //
        // Sprite resources
        //
        let sprite_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("f8c4897e-7c1d-4736-93b7-f2deda158ec7"),
            asset_resource,
        );

        //
        // Debug resources
        //
        let debug_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("11d3b144-f564-42c9-b31f-82c8a938bf85"),
            asset_resource,
        );

        //
        // Bloom extract resources
        //
        let bloom_extract_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("822c8e08-2720-4002-81da-fd9c4d61abdd"),
            asset_resource,
        );

        //
        // Bloom blur resources
        //
        let bloom_blur_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("22aae4c1-fd0f-414a-9de1-7f68bdf1bfb1"),
            asset_resource,
        );

        //
        // Bloom combine resources
        //
        let bloom_combine_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("256e6a2d-669b-426b-900d-3bcc4249a063"),
            asset_resource,
        );

        //
        // Mesh resources
        //
        let mesh_material = begin_load_asset::<MaterialAsset>(
            asset_uuid!("267e0388-2611-441c-9c78-2d39d1bd3cf1"),
            asset_resource,
        );

        wait_for_asset_to_load(
            &sprite_material,
            asset_resource,
            resource_manager,
            "sprite_material"
        );

        wait_for_asset_to_load(
            &debug_material,
            asset_resource,
            resource_manager,
            "debub material"
        );

        wait_for_asset_to_load(
            &bloom_extract_material,
            asset_resource,
            resource_manager,
            "bloom extract material"
        );

        wait_for_asset_to_load(
            &bloom_blur_material,
            asset_resource,
            resource_manager,
            "bloom blur material"
        );

        wait_for_asset_to_load(
            &bloom_combine_material,
            asset_resource,
            resource_manager,
            "bloom combine material"
        );

        wait_for_asset_to_load(
            &mesh_material,
            asset_resource,
            resource_manager,
            "mesh material"
        );

        Ok(GameRendererStaticResources {
            sprite_material,
            debug_material,
            mesh_material,
            bloom_extract_material,
            bloom_blur_material,
            bloom_combine_material,
        })
    }
}
