use super::*;
use crate::assets::mesh_adv::MeshAdvAsset;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use crate::shaders::mesh_adv::mesh_adv_textured_frag;
use fnv::FnvHashMap;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{
    BufferResource, DescriptorSetArc, ImageViewResource, MaterialPassResource, ResourceArc,
};
use rafx::rafx_visibility::geometry::Transform;

pub struct MeshAdvRenderFeatureTypes;

//TODO: Pull this const from the shader
pub const MAX_SHADOW_MAPS_2D: usize = 96;
pub const MAX_SHADOW_MAPS_CUBE: usize = 32;

//---------
// EXTRACT
//---------

pub struct MeshAdvPerFrameData {
    pub depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub shadow_map_atlas_depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub shadow_map_atlas: ResourceArc<ImageViewResource>,
}

pub struct MeshAdvRenderObjectInstanceData {
    pub mesh_asset: MeshAdvAsset,
    pub transform: Transform,
    pub previous_transform: Option<Transform>,
}

#[derive(Default)]
pub struct MeshAdvPerViewData {
    //TODO: Replace with arrayvec/tinyvec? These were static arrays but they can get big enough now
    // that working with them as static arrays is difficult
    pub directional_lights: Vec<ExtractedDirectionalLight>,
    pub point_lights: Vec<ExtractedPointLight>,
    pub spot_lights: Vec<ExtractedSpotLight>,
    pub ndf_filter_amount: f32,
    pub ambient_light: glam::Vec3,
    pub use_clustered_lighting: bool,
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

impl FramePacketData for MeshAdvRenderFeatureTypes {
    type PerFrameData = MeshAdvPerFrameData;
    type RenderObjectInstanceData = Option<MeshAdvRenderObjectInstanceData>;
    type PerViewData = MeshAdvPerViewData;
    type RenderObjectInstancePerViewData = ();
}

pub type MeshAdvFramePacket = FramePacket<MeshAdvRenderFeatureTypes>;

//---------
// PREPARE
//---------

#[derive(Clone)]
pub struct MeshAdvPartMaterialDescriptorSetPair {
    pub textured_descriptor_set: Option<DescriptorSetArc>,
    pub untextured_descriptor_set: Option<DescriptorSetArc>,
}

pub struct MeshAdvPerFrameSubmitData {
    pub num_shadow_map_2d: usize,
    pub shadow_map_2d_data: [mesh_adv_textured_frag::ShadowMap2DDataStd140; MAX_SHADOW_MAPS_2D],
    pub num_shadow_map_cube: usize,
    pub shadow_map_cube_data:
        [mesh_adv_textured_frag::ShadowMapCubeDataStd140; MAX_SHADOW_MAPS_CUBE],
    // Remaps from shadow view index (used to index into the data of MeshAdvShadowMapResource) to the array index in the shader uniform
    pub shadow_map_image_index_remap: FnvHashMap<ShadowViewIndex, usize>,
    pub model_matrix_buffer: TrustCell<Option<ResourceArc<BufferResource>>>,
    pub model_matrix_with_history_buffer: TrustCell<Option<ResourceArc<BufferResource>>>,
}

pub struct MeshAdvRenderObjectInstanceSubmitData {
    pub model_matrix_offset: usize,
}

impl SubmitPacketData for MeshAdvRenderFeatureTypes {
    type PerFrameSubmitData = Box<MeshAdvPerFrameSubmitData>;
    type RenderObjectInstanceSubmitData = MeshAdvRenderObjectInstanceSubmitData;
    type PerViewSubmitData = MeshAdvPerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = MeshAdvDrawCall;

    type RenderFeature = MeshAdvRenderFeature;
}

pub type MeshSubmitPacket = SubmitPacket<MeshAdvRenderFeatureTypes>;

//-------
// WRITE
//-------

pub struct MeshAdvPerViewSubmitData {
    pub opaque_descriptor_set: Option<DescriptorSetArc>,
    pub depth_descriptor_set: Option<DescriptorSetArc>,
    pub shadow_map_atlas_depth_descriptor_set: Option<DescriptorSetArc>,
}

pub struct MeshAdvDrawCall {
    pub render_object_instance_id: RenderObjectInstanceId,
    pub material_pass_resource: ResourceArc<MaterialPassResource>,
    pub per_material_descriptor_set: Option<DescriptorSetArc>,
    pub mesh_part_index: usize,
    pub model_matrix_index: usize,
}
