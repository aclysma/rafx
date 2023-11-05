use crate::assets::font::FontAssetType;
use rafx::assets::AssetManager;
use rafx::assets::AssetResource;
use rafx::framework::RenderResources;
use rafx::renderer::RendererAssetPlugin;
use rafx::RafxResult;

pub struct FontAssetTypeRendererPlugin;

impl RendererAssetPlugin for FontAssetTypeRendererPlugin {
    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _render_resources: &mut RenderResources,
    ) -> RafxResult<()> {
        let asset_type = FontAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)
    }
}
