use crate::assets::mesh_adv::{MeshAdvAssetType, ModelAdvAssetType, PrefabAdvAssetType};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::renderer::RendererAssetPlugin;

pub struct MeshAdvAssetTypeRendererPlugin;

impl RendererAssetPlugin for MeshAdvAssetTypeRendererPlugin {
    //
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon
            .with_importer(&["gltf"], super::MeshAdvGltfImporter)
            .with_importer(&["glb"], super::MeshAdvGltfImporter)
            .with_importer(&["blender_material"], super::MeshAdvBlenderMaterialImporter)
            .with_importer(&["blender_model"], super::MeshAdvBlenderModelImporter)
            .with_importer(&["blender_mesh"], super::MeshAdvBlenderImporter)
            .with_importer(&["blender_prefab"], super::MeshAdvBlenderPrefabImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) {
        asset_manager.register_asset_type::<MeshAdvAssetType>(asset_resource);
        asset_manager.register_asset_type::<ModelAdvAssetType>(asset_resource);
        asset_manager.register_asset_type::<PrefabAdvAssetType>(asset_resource);
    }
}
