use rafx::render_feature_extract_job_predule::*;

use super::*;
use crate::assets::mesh_adv::MeshAdvShaderPassIndices;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use legion::{Entity, IntoQuery, Read, World};
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_map::ReadBorrow;
use rafx::base::resource_ref_map::ResourceRefBorrow;
use rafx::distill::loader::handle::Handle;

pub struct MeshAdvExtractJob<'extract> {
    world: ResourceRefBorrow<'extract, World>,
    mesh_render_options: Option<ResourceRefBorrow<'extract, MeshAdvRenderOptions>>,
    shadow_map_atlas: ReadBorrow<'extract, ShadowMapAtlas>,
    asset_manager: AssetManagerExtractRef,
    default_pbr_material: Handle<MaterialAsset>,
    depth_material: Handle<MaterialAsset>,
    shadow_map_atlas_depth_material: Handle<MaterialAsset>,
    render_objects: MeshAdvRenderObjectSet,
}

impl<'extract> MeshAdvExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<MeshAdvFramePacket>,
        default_pbr_material: Handle<MaterialAsset>,
        depth_material: Handle<MaterialAsset>,
        shadow_map_atlas_depth_material: Handle<MaterialAsset>,
        render_objects: MeshAdvRenderObjectSet,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                world: extract_context.extract_resources.fetch::<World>(),
                mesh_render_options: extract_context
                    .extract_resources
                    .try_fetch::<MeshAdvRenderOptions>(),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                shadow_map_atlas: extract_context.render_resources.fetch::<ShadowMapAtlas>(),
                default_pbr_material,
                depth_material,
                shadow_map_atlas_depth_material,
                render_objects,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for MeshAdvExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        let default_pbr_material = self
            .asset_manager
            .committed_asset(&self.default_pbr_material)
            .unwrap()
            .clone();
        let default_pbr_material_pass_indices =
            MeshAdvShaderPassIndices::new(&default_pbr_material);
        context
            .frame_packet()
            .per_frame_data()
            .set(MeshAdvPerFrameData {
                default_pbr_material,
                default_pbr_material_pass_indices,
                depth_material_pass: self
                    .asset_manager
                    .committed_asset(&self.depth_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
                shadow_map_atlas_depth_material_pass: self
                    .asset_manager
                    .committed_asset(&self.shadow_map_atlas_depth_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
                shadow_map_atlas: self.shadow_map_atlas.shadow_atlas_image_view().clone(),
            });
    }

    fn extract_render_object_instance(
        &self,
        job_context: &mut RenderObjectsJobContext<'extract, MeshAdvRenderObject>,
        context: &ExtractRenderObjectInstanceContext<'extract, '_, Self>,
    ) {
        let render_object_static_data = job_context
            .render_objects
            .get_id(context.render_object_id());

        let mesh_asset = self
            .asset_manager
            .committed_asset(&render_object_static_data.mesh);

        let visibility_info = context.visibility_object_info();
        let transform = visibility_info.transform();
        let previous_transform = visibility_info.previous_frame_transform();

        context.set_render_object_instance_data(mesh_asset.and_then(|mesh_asset| {
            Some(MeshAdvRenderObjectInstanceData {
                mesh_asset: mesh_asset.clone(),
                transform,
                previous_transform,
            })
        }));
    }

    fn end_per_view_extract(
        &self,
        context: &ExtractPerViewContext<'extract, '_, Self>,
    ) {
        let mut per_view = MeshAdvPerViewData::default();
        let is_lit = !context
            .view()
            .feature_flag_is_relevant::<MeshAdvUnlitRenderFeatureFlag>();

        if !is_lit {
            context.view_packet().per_view_data().set(per_view);
            return;
        }

        let world = &*self.world;

        per_view.directional_lights.reserve(32);
        let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
        for light in query.iter(world).map(|(e, l)| ExtractedDirectionalLight {
            object_id: ObjectId::from(*e),
            light: l.clone(),
        }) {
            per_view.directional_lights.push(light);
        }

        per_view.point_lights.reserve(512);
        let mut query = <(Entity, Read<TransformComponent>, Read<PointLightComponent>)>::query();
        for light in query.iter(world).map(|(e, p, l)| ExtractedPointLight {
            object_id: ObjectId::from(*e),
            light: l.clone(),
            transform: p.clone(),
        }) {
            per_view.point_lights.push(light);
        }

        per_view.spot_lights.reserve(512);
        let mut query = <(Entity, Read<TransformComponent>, Read<SpotLightComponent>)>::query();
        for light in query.iter(world).map(|(e, p, l)| ExtractedSpotLight {
            object_id: ObjectId::from(*e),
            light: l.clone(),
            transform: p.clone(),
        }) {
            per_view.spot_lights.push(light);
        }

        if let Some(mesh_render_options) = &self.mesh_render_options {
            per_view.ambient_light = mesh_render_options.ambient_light;
            per_view.ndf_filter_amount = mesh_render_options.ndf_filter_amount;
            per_view.use_clustered_lighting = mesh_render_options.use_clustered_lighting;
        } else {
            per_view.ambient_light = glam::Vec3::ZERO;
            per_view.ndf_filter_amount = 1.0;
            per_view.use_clustered_lighting = true;
        }

        context.view_packet().per_view_data().set(per_view);
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    fn new_render_object_instance_job_context(
        &'extract self
    ) -> Option<RenderObjectsJobContext<'extract, MeshAdvRenderObject>> {
        Some(RenderObjectsJobContext::new(self.render_objects.read()))
    }

    type RenderObjectInstanceJobContextT = RenderObjectsJobContext<'extract, MeshAdvRenderObject>;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = MeshAdvRenderFeatureTypes;
}
