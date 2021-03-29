use crate::assets::ldtk::LdtkAssetType;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::renderer::RendererPlugin;

pub struct LdtkAssetTypeRendererPlugin;

impl RendererPlugin for LdtkAssetTypeRendererPlugin {
    //
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon.with_importer("ldtk", super::LdtkImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) {
        asset_manager.register_asset_type::<LdtkAssetType>(asset_resource);
    }
}
