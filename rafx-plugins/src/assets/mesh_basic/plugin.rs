use crate::assets::mesh_basic::{
    MeshBasicAssetType, MeshMaterialBasicAssetType, ModelBasicAssetType, PrefabBasicAssetType,
};
use rafx::assets::AssetManager;
use rafx::assets::AssetResource;
use rafx::distill::daemon::AssetDaemon;
use rafx::framework::RenderResources;
use rafx::renderer::RendererAssetPlugin;
use rafx::RafxResult;

pub struct MeshBasicAssetTypeRendererPlugin;

impl RendererAssetPlugin for MeshBasicAssetTypeRendererPlugin {
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
