use crate::assets::ldtk::LdtkAssetType;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::framework::RenderResources;
use rafx::renderer::RendererAssetPlugin;
use rafx::RafxResult;

pub struct LdtkAssetTypeRendererPlugin;

impl RendererAssetPlugin for LdtkAssetTypeRendererPlugin {
    //
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon.with_importer(&["ldtk"], super::LdtkImporter)
    }

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
