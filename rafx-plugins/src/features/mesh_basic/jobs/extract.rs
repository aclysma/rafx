use rafx::render_feature_extract_job_predule::*;

use super::*;
use crate::assets::mesh_basic::MeshBasicShaderPassIndices;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use legion::{Entity, IntoQuery, Read, World};
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_ref_map::ResourceRefBorrow;
use rafx::distill::loader::handle::Handle;

pub struct MeshBasicExtractJob<'extract> {
    world: ResourceRefBorrow<'extract, World>,
    mesh_render_options: Option<ResourceRefBorrow<'extract, MeshBasicRenderOptions>>,
    asset_manager: AssetManagerExtractRef,
    default_pbr_material: Handle<MaterialAsset>,
    depth_material: Handle<MaterialAsset>,
    render_objects: MeshBasicRenderObjectSet,
}

impl<'extract> MeshBasicExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<MeshBasicFramePacket>,
        default_pbr_material: Handle<MaterialAsset>,
        depth_material: Handle<MaterialAsset>,
        render_objects: MeshBasicRenderObjectSet,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                world: extract_context.extract_resources.fetch::<World>(),
                mesh_render_options: extract_context
                    .extract_resources
                    .try_fetch::<MeshBasicRenderOptions>(),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                default_pbr_material,
                depth_material,
                render_objects,
            },
            extract_context,
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for MeshBasicExtractJob<'extract> {
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
            MeshBasicShaderPassIndices::new(&default_pbr_material);
        context
            .frame_packet()
            .per_frame_data()
            .set(MeshBasicPerFrameData {
                default_pbr_material,
                default_pbr_material_pass_indices,
                depth_material_pass: self
                    .asset_manager
                    .committed_asset(&self.depth_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
            });
    }

    fn extract_render_object_instance(
        &self,
        job_context: &mut RenderObjectsJobContext<'extract, MeshBasicRenderObject>,
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

        context.set_render_object_instance_data(mesh_asset.and_then(|mesh_asset| {
            Some(MeshBasicRenderObjectInstanceData {
                mesh_asset: mesh_asset.clone(),
                transform,
            })
        }));
    }

    fn end_per_view_extract(
        &self,
        context: &ExtractPerViewContext<'extract, '_, Self>,
    ) {
        let mut per_view = MeshBasicPerViewData::default();
        let is_lit = !context
            .view()
            .feature_flag_is_relevant::<MeshBasicUnlitRenderFeatureFlag>();

        if !is_lit {
            context.view_packet().per_view_data().set(per_view);
            return;
        }

        let world = &*self.world;

        let mut query = <(Entity, Read<DirectionalLightComponent>)>::query();
        for light in query.iter(world).map(|(e, l)| ExtractedDirectionalLight {
            object_id: ObjectId::from(*e),
            light: l.clone(),
        }) {
            let next_index = per_view.num_directional_lights;
            if per_view.directional_lights.len() > next_index as usize {
                per_view.directional_lights[next_index as usize] = Some(light);
                per_view.num_directional_lights += 1;
            }
        }

        let mut query = <(Entity, Read<TransformComponent>, Read<PointLightComponent>)>::query();
        for light in query.iter(world).map(|(e, p, l)| ExtractedPointLight {
            object_id: ObjectId::from(*e),
            light: l.clone(),
            transform: p.clone(),
        }) {
            let next_index = per_view.num_point_lights;
            if per_view.point_lights.len() > next_index as usize {
                per_view.point_lights[next_index as usize] = Some(light);
                per_view.num_point_lights += 1;
            }
        }

        let mut query = <(Entity, Read<TransformComponent>, Read<SpotLightComponent>)>::query();
        for light in query.iter(world).map(|(e, p, l)| ExtractedSpotLight {
            object_id: ObjectId::from(*e),
            light: l.clone(),
            transform: p.clone(),
        }) {
            let next_index = per_view.num_spot_lights;
            if per_view.spot_lights.len() > next_index as usize {
                per_view.spot_lights[next_index as usize] = Some(light);
                per_view.num_spot_lights += 1;
            }
        }

        if let Some(mesh_render_options) = &self.mesh_render_options {
            per_view.ambient_light = mesh_render_options.ambient_light;
        } else {
            per_view.ambient_light = glam::Vec3::ZERO;
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
    ) -> Option<RenderObjectsJobContext<'extract, MeshBasicRenderObject>> {
        Some(RenderObjectsJobContext::new(self.render_objects.read()))
    }

    type RenderObjectInstanceJobContextT = RenderObjectsJobContext<'extract, MeshBasicRenderObject>;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = MeshBasicRenderFeatureTypes;
}
