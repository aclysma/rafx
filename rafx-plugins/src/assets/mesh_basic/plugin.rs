use crate::assets::mesh_basic::{MeshBasicAssetType, ModelBasicAssetType, PrefabBasicAssetType};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::renderer::RendererAssetPlugin;

pub struct MeshBasicAssetTypeRendererPlugin;

impl RendererAssetPlugin for MeshBasicAssetTypeRendererPlugin {
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon
            .with_importer(&["gltf"], super::MeshBasicGltfImporter)
            .with_importer(&["glb"], super::MeshBasicGltfImporter)
            .with_importer(
                &["blender_material"],
                super::MeshBasicBlenderMaterialImporter,
            )
            .with_importer(&["blender_model"], super::MeshBasicBlenderModelImporter)
            .with_importer(&["blender_mesh"], super::MeshBasicBlenderImporter)
            .with_importer(&["blender_prefab"], super::MeshBasicBlenderPrefabImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) {
        asset_manager.register_asset_type::<MeshBasicAssetType>(asset_resource);
        asset_manager.register_asset_type::<ModelBasicAssetType>(asset_resource);
        asset_manager.register_asset_type::<PrefabBasicAssetType>(asset_resource);
    }
}
