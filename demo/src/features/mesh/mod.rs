use rafx::render_feature_mod_prelude::*;
use rafx::render_feature_renderer_prelude::*;
rafx::declare_render_feature!(MeshRenderFeature, MESH_FEATURE_INDEX);

mod extract;
use extract::*;
mod prepare;
use prepare::*;
mod write;
use write::*;
mod public;
pub use public::*;

use distill::loader::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::nodes::{FramePacketBuilder, RenderView, RenderViewSet};
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};

struct StaticResources {
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
        render_registry.register_feature::<RenderFeatureType>()
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

        render_resources.insert(StaticResources { depth_material });

        render_resources.insert(ShadowMapResource::default());

        Ok(())
    }

    fn add_render_views(
        &self,
        extract_resources: &ExtractResources,
        render_resources: &RenderResources,
        render_view_set: &RenderViewSet,
        frame_packet_builder: &FramePacketBuilder,
        static_visibility_node_set: &mut StaticVisibilityNodeSet,
        dynamic_visibility_node_set: &mut DynamicVisibilityNodeSet,
        render_views: &mut Vec<RenderView>,
    ) {
        let mut shadow_map_resource = render_resources.fetch_mut::<ShadowMapResource>();
        shadow_map_resource.recalculate_shadow_map_views(
            &render_view_set,
            extract_resources,
            &frame_packet_builder,
            static_visibility_node_set,
            dynamic_visibility_node_set,
        );

        shadow_map_resource.append_render_views(render_views);
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(Box::new(ExtractJobImpl::new()));
    }
}
