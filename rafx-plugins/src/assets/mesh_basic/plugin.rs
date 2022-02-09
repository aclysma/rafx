use crate::assets::mesh_basic::{
    MeshBasicAssetType, MeshMaterialBasicAssetType, ModelBasicAssetType, PrefabBasicAssetType,
};
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::framework::RenderResources;
use rafx::renderer::RendererAssetPlugin;
use rafx::RafxResult;

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
        _render_resources: &mut RenderResources,
    ) -> RafxResult<()> {
        let asset_type = MeshMaterialBasicAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)?;
        let asset_type = MeshBasicAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)?;
        let asset_type = ModelBasicAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)?;
        let asset_type = PrefabBasicAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)?;
        Ok(())
    }
}
