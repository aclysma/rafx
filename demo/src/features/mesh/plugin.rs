use rafx::render_feature_renderer_prelude::*;

use super::{MeshExtractJob, MeshRenderFeature, MeshRenderNodeSet, ShadowMapResource};
use distill::loader::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::nodes::{FramePacketBuilder, RenderView, RenderViewSet};
use rafx::renderer::RendererConfigResource;
use rafx::visibility::VisibilityRegion;

pub struct MeshStaticResources {
    pub depth_material: Handle<MaterialAsset>,
}

pub struct MeshRendererPlugin;

impl MeshRendererPlugin {
    pub fn legion_init(resources: &mut legion::Resources) {
        resources.insert(MeshRenderNodeSet::default());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<MeshRenderNodeSet>();
    }
}

impl RendererPlugin for MeshRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<MeshRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let depth_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/depth.material");

        asset_manager.wait_for_asset_to_load(&depth_material, asset_resource, "depth")?;

        render_resources.insert(MeshStaticResources { depth_material });

        render_resources.insert(ShadowMapResource::default());

        Ok(())
    }

    fn add_render_views(
        &self,
        extract_resources: &ExtractResources,
        render_resources: &RenderResources,
        render_view_set: &RenderViewSet,
        frame_packet_builder: &FramePacketBuilder,
        render_views: &mut Vec<RenderView>,
    ) {
        let mut shadow_map_resource = render_resources.fetch_mut::<ShadowMapResource>();
        let visibility_region = extract_resources.fetch::<VisibilityRegion>();
        let renderer_config = extract_resources
            .try_fetch::<RendererConfigResource>()
            .map(|x| *x)
            .unwrap_or_default();

        shadow_map_resource.recalculate_shadow_map_views(
            &render_view_set,
            extract_resources,
            &visibility_region,
            &renderer_config.visibility_config,
            &frame_packet_builder,
        );

        shadow_map_resource.append_render_views(render_views);
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(Box::new(MeshExtractJob::new()));
    }
}
