use crate::features::mesh::shadow_map_resource::ShadowMapResource;
use crate::features::mesh::{MeshRenderFeature, MeshRenderNodeSet};
use rafx::api::extra::upload::RafxTransferUpload;
use rafx::api::RafxResult;
use rafx::assets::distill_impl::AssetResource;
use rafx::assets::AssetManager;
use rafx::base::resource_map::ResourceMap;
use rafx::framework::RenderResources;
use rafx::nodes::{
    ExtractJob, ExtractResources, FramePacketBuilder, RenderNodeReservations,
    RenderRegistryBuilder, RenderView, RenderViewSet,
};
use rafx::renderer::RendererPlugin;
use rafx::visibility::{DynamicVisibilityNodeSet, StaticVisibilityNodeSet};

pub struct MeshRendererPlugin;

impl RendererPlugin for MeshRendererPlugin {
    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<MeshRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        _asset_manager: &mut AssetManager,
        _asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        render_resources.insert(ShadowMapResource::default());

        Ok(())
    }

    fn add_render_node_reservations(
        &self,
        render_node_reservations: &mut RenderNodeReservations,
        extract_resources: &ExtractResources,
    ) {
        let mut mesh_render_nodes = extract_resources.fetch_mut::<MeshRenderNodeSet>();
        mesh_render_nodes.update();
        render_node_reservations.add_reservation(&*mesh_render_nodes);
    }

    fn add_extract_jobs(
        &self,
        _extract_resources: &ExtractResources,
        _render_resources: &RenderResources,
        extract_jobs: &mut Vec<Box<dyn ExtractJob>>,
    ) {
        extract_jobs.push(super::create_mesh_extract_job());
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
}
