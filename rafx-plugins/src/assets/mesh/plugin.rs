use crate::assets::mesh::{MeshAssetType, ModelAssetType, PrefabAssetType};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::renderer::RendererAssetPlugin;

pub struct GltfAssetTypeRendererPlugin;

impl RendererAssetPlugin for GltfAssetTypeRendererPlugin {
    //
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon
            .with_importer(&["gltf"], super::GltfImporter)
            .with_importer(&["glb"], super::GltfImporter)
            .with_importer(&["blender_material"], super::BlenderMaterialImporter)
            .with_importer(&["blender_model"], super::BlenderModelImporter)
            .with_importer(&["blender_mesh"], super::BlenderMeshImporter)
            .with_importer(&["blender_prefab"], super::BlenderPrefabImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) {
        asset_manager.register_asset_type::<MeshAssetType>(asset_resource);
        asset_manager.register_asset_type::<ModelAssetType>(asset_resource);
        asset_manager.register_asset_type::<PrefabAssetType>(asset_resource);
    }
}
