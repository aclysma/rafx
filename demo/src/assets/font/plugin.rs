use crate::assets::font::FontAssetType;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::renderer::RendererPlugin;

pub struct FontAssetTypeRendererPlugin;

impl RendererPlugin for FontAssetTypeRendererPlugin {
    //
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon.with_importer("ttf", super::FontImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) {
        asset_manager.register_asset_type::<FontAssetType>(asset_resource);
    }
}
