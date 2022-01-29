use crate::assets::anim::AnimAssetType;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::distill::daemon::AssetDaemon;
use rafx::framework::RenderResources;
use rafx::renderer::RendererAssetPlugin;
use rafx::RafxResult;

pub struct AnimAssetTypeRendererPlugin;

impl RendererAssetPlugin for AnimAssetTypeRendererPlugin {
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon.with_importer(&["blender_anim"], super::BlenderAnimImporter)
    }

    fn register_asset_types(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _render_resources: &mut RenderResources,
    ) -> RafxResult<()> {
        let asset_type = AnimAssetType::create(asset_manager, asset_resource)?;
        asset_manager.register_asset_type(asset_type)
    }
}
