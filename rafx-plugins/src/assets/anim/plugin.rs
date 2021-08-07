use crate::assets::anim::AnimAssetType;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::renderer::RendererAssetPlugin;

pub struct AnimAssetTypeRendererPlugin;

impl RendererAssetPlugin for AnimAssetTypeRendererPlugin {
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon.with_importer("blender_anim", super::BlenderAnimImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
    ) {
        asset_manager.register_asset_type::<AnimAssetType>(asset_resource);
    }
}
