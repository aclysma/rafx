use crate::RendererLoadContext;
use rafx_api::extra::upload::RafxTransferUpload;
use rafx_api::RafxResult;
use rafx_assets::distill_impl::AssetResource;
use rafx_assets::AssetManager;
use rafx_framework::graph::PreparedRenderGraph;
use rafx_framework::render_features::{ExtractResources, RenderRegistryBuilder, RenderView};
use rafx_framework::{ImageViewResource, RenderResources, ResourceArc};
use std::path::PathBuf;

pub trait RendererPipelinePlugin: Send + Sync {
    fn plugin_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn add_asset_paths(
        &self,
        _asset_paths: &mut Vec<PathBuf>,
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
        _renderer_load_context: &RendererLoadContext,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        _render_resources: &mut RenderResources,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        Ok(())
    }

    fn generate_render_graph(
        &self,
        asset_manager: &AssetManager,
        swapchain_image: ResourceArc<ImageViewResource>,
        rotating_frame_index: usize,
        main_view: RenderView,
        extract_resources: &ExtractResources,
        render_resources: &RenderResources,
    ) -> RafxResult<PreparedRenderGraph>;

    fn finish_frame(
        &self,
        _render_resources: &RenderResources,
    ) {
    }

    fn prepare_renderer_destroy(
        &self,
        _render_resources: &RenderResources,
    ) -> RafxResult<()> {
        Ok(())
    }
}
