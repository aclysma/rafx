use crate::RendererLoadContext;
use rafx_api::extra::upload::RafxTransferUpload;
use rafx_api::RafxResult;
use rafx_assets::distill::daemon::AssetDaemon;
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::AssetManager;
use rafx_framework::render_features::{ExtractResources, RenderRegistryBuilder};
use rafx_framework::RenderResources;
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
        _render_resources: &mut RenderResources,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry
    }

    fn initialize_static_resources(
        &self,
        _renderer_load_context: &RendererLoadContext,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        _render_resources: &mut RenderResources,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn process_asset_loading(
        &self,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        _render_resources: &RenderResources,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn on_frame_complete(
        &self,
        _asset_manager: &mut AssetManager,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn prepare_renderer_destroy(
        &self,
        _render_resources: &RenderResources,
    ) -> RafxResult<()> {
        Ok(())
    }
}
