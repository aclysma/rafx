use super::*;
use crate::assets::gltf::MeshAsset;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use glam::{Quat, Vec3};
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{
    DescriptorSetArc, DescriptorSetLayoutResource, ImageViewResource, MaterialPassResource,
    ResourceArc,
};
use std::sync::Arc;

pub struct MeshRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub type MeshRenderObjectStaticData = MeshRenderObject;

pub struct MeshPerFrameData {
    pub depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
}

pub struct MeshRenderObjectInstanceData {
    pub mesh_asset: MeshAsset,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Default)]
pub struct MeshPerViewData {
    pub directional_lights: [Option<ExtractedDirectionalLight>; 16],
    pub point_lights: [Option<ExtractedPointLight>; 16],
    pub spot_lights: [Option<ExtractedSpotLight>; 16],
    pub num_directional_lights: u32,
    pub num_point_lights: u32,
    pub num_spot_lights: u32,
}

pub struct ExtractedDirectionalLight {
    pub light: DirectionalLightComponent,
    pub object_id: ObjectId,
}

pub struct ExtractedPointLight {
    pub light: PointLightComponent,
    pub transform: TransformComponent,
    pub object_id: ObjectId,
}

pub struct ExtractedSpotLight {
    pub light: SpotLightComponent,
    pub transform: TransformComponent,
    pub object_id: ObjectId,
}

impl FramePacketData for MeshRenderFeatureTypes {
    type PerFrameData = MeshPerFrameData;
    type RenderObjectInstanceData = Option<MeshRenderObjectInstanceData>;
    type PerViewData = MeshPerViewData;
    type RenderObjectInstancePerViewData = ();
}

pub type MeshFramePacket = FramePacket<MeshRenderFeatureTypes>;
pub type MeshViewPacket = ViewPacket<MeshRenderFeatureTypes>;

//---------
// PREPARE
//---------

//TODO: Pull this const from the shader
pub const MAX_SHADOW_MAPS_2D: usize = 32;
pub const MAX_SHADOW_MAPS_CUBE: usize = 16;

pub struct MeshPartDescriptorSetPair {
    pub depth_descriptor_set: DescriptorSetArc,
    pub opaque_descriptor_set: DescriptorSetArc,
}

pub struct MeshPerFrameSubmitData {
    pub num_shadow_map_2d: usize,
    pub shadow_map_2d_data: [shaders::mesh_frag::ShadowMap2DDataStd140; MAX_SHADOW_MAPS_2D],
    pub shadow_map_2d_image_views: [Option<ResourceArc<ImageViewResource>>; MAX_SHADOW_MAPS_2D],
    pub num_shadow_map_cube: usize,
    pub shadow_map_cube_data: [shaders::mesh_frag::ShadowMapCubeDataStd140; MAX_SHADOW_MAPS_CUBE],
    pub shadow_map_cube_image_views: [Option<ResourceArc<ImageViewResource>>; MAX_SHADOW_MAPS_CUBE],
    pub shadow_map_image_index_remap: [Option<usize>; MAX_SHADOW_MAPS_2D + MAX_SHADOW_MAPS_CUBE],
    pub mesh_part_descriptor_sets: Arc<AtomicOnceCellStack<MeshPartDescriptorSetPair>>,
    pub opaque_per_view_descriptor_set_layout: Option<ResourceArc<DescriptorSetLayoutResource>>,
}

pub struct MeshRenderObjectInstanceSubmitData {
    pub mesh_part_descriptor_set_index: usize,
}

impl SubmitPacketData for MeshRenderFeatureTypes {
    type PerFrameSubmitData = Box<MeshPerFrameSubmitData>;
    type RenderObjectInstanceSubmitData = MeshRenderObjectInstanceSubmitData;
    type PerViewSubmitData = MeshPerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = MeshDrawCall;

    type RenderFeature = MeshRenderFeature;
}

pub type MeshSubmitPacket = SubmitPacket<MeshRenderFeatureTypes>;
pub type MeshViewSubmitPacket = ViewSubmitPacket<MeshRenderFeatureTypes>;

//-------
// WRITE
//-------

pub struct MeshPerViewSubmitData {
    pub opaque_descriptor_set: Option<DescriptorSetArc>,
    pub depth_descriptor_set: Option<DescriptorSetArc>,
}

pub struct MeshDrawCall {
    pub mesh_asset: MeshAsset,
    pub mesh_part_index: usize,
    pub mesh_part_descriptor_set_index: usize,
}
