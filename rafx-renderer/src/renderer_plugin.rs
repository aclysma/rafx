use rafx_api::extra::upload::RafxTransferUpload;
use rafx_api::RafxResult;
use rafx_assets::distill::daemon::AssetDaemon;
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::AssetManager;
use rafx_base::resource_map::ResourceMap;
use rafx_framework::nodes::{
    ExtractJob, ExtractResources, FramePacketBuilder, RenderNodeReservations,
    RenderRegistryBuilder, RenderView, RenderViewSet,
};
use rafx_framework::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};
use rafx_framework::RenderResources;

pub trait RendererPlugin: Send + Sync {
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

    // fn swapchain_created(
    //     &self,
    //     _extract_resources: &ExtractResources,
    // ) -> RafxResult<()> {
    //     Ok(())
    // }
    //
    // fn swapchain_destroyed(
    //     &self,
    //     _extract_resources: &ExtractResources,
    // ) -> RafxResult<()> {
    //     Ok(())
    // }

    fn add_render_node_reservations(
        &self,
        _render_node_reservations: &mut RenderNodeReservations,
        _extract_resources: &ExtractResources,
    ) {
    }

    fn add_render_views(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        _render_view_set: &RenderViewSet,
        _frame_packet_builder: &FramePacketBuilder,
        _static_visibility_node_set: &mut StaticVisibilityNodeSet,
        _dynamic_visibility_node_set: &mut DynamicVisibilityNodeSet,
        _render_views: &mut Vec<RenderView>,
    ) {
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        _extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
    }
}
