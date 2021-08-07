use rafx_api::extra::upload::RafxTransferUpload;
use rafx_api::RafxResult;
use rafx_assets::distill::daemon::AssetDaemon;
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::AssetManager;
use rafx_base::resource_map::ResourceMap;
use rafx_framework::render_features::{ExtractResources, RenderRegistryBuilder};
use std::path::PathBuf;

pub trait RendererAssetPlugin: Send + Sync {
    fn plugin_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn add_asset_paths(
        &self,
        _asset_paths: &mut Vec<PathBuf>,
    ) {
    }

    // If the daemon is not running in-process, this will not be called
    fn configure_asset_daemon(
        &self,
        asset_daemon: AssetDaemon,
    ) -> AssetDaemon {
        asset_daemon
    }

    fn register_asset_types(
        &self,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
    ) {
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry
    }

    fn initialize_static_resources(
        &self,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        _render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        Ok(())
    }
}
