use crate::components::MeshComponent;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::features::mesh::prepare::MeshPrepareJob;
use crate::features::mesh::{
    ExtractedDirectionalLight, ExtractedFrameNodeMeshData, ExtractedPointLight, ExtractedSpotLight,
    MeshRenderFeature, MeshRenderNode, MeshRenderNodeSet,
};
use legion::*;
use rafx::assets::AssetManagerRenderResource;
use rafx::base::slab::RawSlabKey;
use rafx::nodes::{
    ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex,
    RenderJobExtractContext, RenderView,
};

pub struct MeshExtractJob {}

impl ExtractJob for MeshExtractJob {
    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }

    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!("Mesh Extract");
        let legion_world = extract_context.extract_resources.fetch::<World>();
        let world = &*legion_world;

        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        //
        // Update the mesh render nodes. This could be done earlier as part of a system
        //
        let mut mesh_render_nodes = extract_context
            .extract_resources
            .fetch_mut::<MeshRenderNodeSet>();

        let mut query = <(Read<PositionComponent>, Read<MeshComponent>)>::query();
        for (position_component, mesh_component) in query.iter(world) {
            let render_node = mesh_render_nodes
                .get_mut(&mesh_component.render_node)
                .unwrap();
            render_node.mesh = mesh_component.mesh.clone();
            render_node.transform = glam::Mat4::from_translation(position_component.position);
        }

        //
        // Get the position/mesh asset pairs we will draw
        //
        let mut extracted_frame_node_mesh_data =
            Vec::<Option<ExtractedFrameNodeMeshData>>::with_capacity(
                frame_packet.frame_node_count(self.feature_index()) as usize,
            );

        for frame_node in frame_packet
            .frame_nodes(MeshRenderFeature::feature_index())
            .iter()
        {
            let render_node_index = frame_node.render_node_index();
            let render_node_handle = RawSlabKey::<MeshRenderNode>::new(render_node_index);
            let mesh_render_node = mesh_render_nodes
                .meshes
                .get_raw(render_node_handle)
                .unwrap();

            let mesh_asset = mesh_render_node
                .mesh
                .as_ref()
                .and_then(|mesh_asset_handle| asset_manager.committed_asset(mesh_asset_handle));

            let extracted_frame_node = mesh_asset.and_then(|mesh_asset| {
                Some(ExtractedFrameNodeMeshData {
                    mesh_asset: mesh_asset.clone(),
                    world_transform: mesh_render_node.transform,
                })
            });

            extracted_frame_node_mesh_data.push(extracted_frame_node);
        }

        //
        // Get the lights
        //
        let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
        let directional_lights = query
            .iter(world)
            .map(|(e, l)| ExtractedDirectionalLight {
                entity: *e,
                light: l.clone(),
            })
            .collect();

        let mut query = <(Entity, Read<PositionComponent>, Read<PointLightComponent>)>::query();
        let point_lights = query
            .iter(world)
            .map(|(e, p, l)| ExtractedPointLight {
                entity: *e,
                light: l.clone(),
                position: p.clone(),
            })
            .collect();

        let mut query = <(Entity, Read<PositionComponent>, Read<SpotLightComponent>)>::query();
        let spot_lights = query
            .iter(world)
            .map(|(e, p, l)| ExtractedSpotLight {
                entity: *e,
                light: l.clone(),
                position: p.clone(),
            })
            .collect();

        Box::new(MeshPrepareJob {
            extracted_frame_node_mesh_data,
            directional_lights,
            point_lights,
            spot_lights,
        })
    }
}
