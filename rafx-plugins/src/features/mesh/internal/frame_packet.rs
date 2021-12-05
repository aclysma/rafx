use super::*;
use crate::assets::mesh::MeshAsset;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use crate::shaders::mesh_basic::mesh_basic_textured_frag;
use glam::{Quat, Vec3};
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{
    BufferResource, DescriptorSetArc, ImageViewResource, MaterialPassResource, ResourceArc,
};

pub struct MeshRenderFeatureTypes;

//TODO: Pull this const from the shader
pub const MAX_SHADOW_MAPS_2D: usize = 32;
pub const MAX_SHADOW_MAPS_CUBE: usize = 16;
pub const MAX_DIRECTIONAL_LIGHTS: usize = 16;
pub const MAX_POINT_LIGHTS: usize = 16;
pub const MAX_SPOT_LIGHTS: usize = 16;

//---------
// EXTRACT
//---------

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
    pub directional_lights: [Option<ExtractedDirectionalLight>; MAX_DIRECTIONAL_LIGHTS],
    pub point_lights: [Option<ExtractedPointLight>; MAX_POINT_LIGHTS],
    pub spot_lights: [Option<ExtractedSpotLight>; MAX_SPOT_LIGHTS],
    pub num_directional_lights: u32,
    pub num_point_lights: u32,
    pub num_spot_lights: u32,
    pub ambient_light: glam::Vec3,
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

//---------
// PREPARE
//---------

#[derive(Clone)]
pub struct MeshPartMaterialDescriptorSetPair {
    pub textured_descriptor_set: Option<DescriptorSetArc>,
    pub untextured_descriptor_set: Option<DescriptorSetArc>,
}

pub struct MeshPerFrameSubmitData {
    pub num_shadow_map_2d: usize,
    pub shadow_map_2d_data: [mesh_basic_textured_frag::ShadowMap2DDataStd140; MAX_SHADOW_MAPS_2D],
    pub shadow_map_2d_image_views: [Option<ResourceArc<ImageViewResource>>; MAX_SHADOW_MAPS_2D],
    pub num_shadow_map_cube: usize,
    pub shadow_map_cube_data:
        [mesh_basic_textured_frag::ShadowMapCubeDataStd140; MAX_SHADOW_MAPS_CUBE],
    pub shadow_map_cube_image_views: [Option<ResourceArc<ImageViewResource>>; MAX_SHADOW_MAPS_CUBE],
    pub shadow_map_image_index_remap: [Option<usize>; MAX_SHADOW_MAPS_2D + MAX_SHADOW_MAPS_CUBE],
    pub model_matrix_buffer: TrustCell<Option<ResourceArc<BufferResource>>>,
}

pub struct MeshRenderObjectInstanceSubmitData {
    pub model_matrix_offset: usize,
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

//-------
// WRITE
//-------

pub struct MeshPerViewSubmitData {
    pub opaque_descriptor_set: Option<DescriptorSetArc>,
    pub depth_descriptor_set: Option<DescriptorSetArc>,
}

pub struct MeshDrawCall {
    pub render_object_instance_id: RenderObjectInstanceId,
    pub material_pass_resource: ResourceArc<MaterialPassResource>,
    pub per_material_descriptor_set: Option<DescriptorSetArc>,
    pub mesh_part_index: usize,
    pub model_matrix_offset: usize,
}
