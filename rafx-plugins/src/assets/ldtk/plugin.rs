use crate::assets::ldtk::LdtkAssetType;
use rafx::assets::AssetManager;
use rafx::assets::AssetResource;
use rafx::framework::RenderResources;
use rafx::renderer::RendererAssetPlugin;
use rafx::RafxResult;

pub struct LdtkAssetTypeRendererPlugin;

impl RendererAssetPlugin for LdtkAssetTypeRendererPlugin {
    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _render_resources: &mut RenderResources,
    ) -> RafxResult<()> {
        let asset_type = LdtkAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)
    }
}
