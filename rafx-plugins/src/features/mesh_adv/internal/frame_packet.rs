use super::*;
use crate::assets::mesh_adv::{MeshAdvAsset, MeshAdvShaderPassIndices};
use crate::components::{
    DirectionalLightComponent, PointLightComponent, SpotLightComponent, TransformComponent,
};
use crate::shaders::mesh_adv::mesh_adv_textured_frag;
use fnv::FnvHashMap;
use rafx::api::RafxIndexType;
use rafx::assets::MaterialAsset;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{
    BufferResource, DescriptorSetArc, ImageViewResource, MaterialPassResource, ResourceArc,
};
use rafx::rafx_visibility::geometry::{BoundingSphere, Transform};

pub struct MeshAdvRenderFeatureTypes;

//TODO: Pull this const from the shader
pub const MAX_SHADOW_MAPS_2D: usize = 96;
pub const MAX_SHADOW_MAPS_CUBE: usize = 32;

//---------
// EXTRACT
//---------

pub struct MeshAdvPerFrameData {
    pub default_pbr_material: MaterialAsset,
    pub default_pbr_material_pass_indices: MeshAdvShaderPassIndices,
    pub depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub shadow_map_atlas_depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub shadow_map_atlas: ResourceArc<ImageViewResource>,
    pub invalid_image_color: ResourceArc<ImageViewResource>,
}

pub struct MeshAdvRenderObjectInstanceData {
    pub mesh_asset: MeshAdvAsset,
    pub transform: Transform,
    pub previous_transform: Option<Transform>,
    pub bounding_sphere: Option<BoundingSphere>,
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

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct MeshAdvBatchedPassKey {
    pub phase: RenderPhaseIndex,
    pub view_frame_index: ViewFrameIndex,
    pub pass: ResourceArc<MaterialPassResource>,
    pub index_type: RafxIndexType,
}

//TODO: We make one per view * object, is this really necessary?
#[derive(Clone)]
pub struct MeshAdvBatchDrawData {
    pub transform_index: u32,
    pub material_index: u32,
    pub vertex_offset: u32, // in number of vertices, not bytes
    pub index_count: u32,   // In number of indices, not bytes
    pub index_offset: u32,  // In number of indices, not bytes
}

pub struct MeshAdvBatchedPassInfo {
    pub phase: RenderPhaseIndex,
    pub pass: ResourceArc<MaterialPassResource>,
    pub draw_data: AtomicOnceCellStack<MeshAdvBatchDrawData>,
    pub view_frame_index: ViewFrameIndex,
    pub index_type: RafxIndexType,
}

#[derive(Clone)]
pub struct MeshAdvBatchedPreparedPassInfo {
    pub phase: RenderPhaseIndex,
    pub pass: ResourceArc<MaterialPassResource>,
    pub index_type: RafxIndexType,
    pub indirect_buffer_first_command_index: u32,
    pub indirect_buffer_command_count: u32,
    pub draw_data: Option<Vec<MeshAdvBatchDrawData>>,
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
    pub all_materials_descriptor_set: TrustCell<Option<DescriptorSetArc>>,
    pub batched_pass_lookup: AtomicOnceCell<FnvHashMap<MeshAdvBatchedPassKey, usize>>,
    pub batched_passes: AtomicOnceCell<Vec<MeshAdvBatchedPreparedPassInfo>>,
    pub per_batch_descriptor_sets: AtomicOnceCell<Vec<Option<DescriptorSetArc>>>,
    pub indirect_buffer: AtomicOnceCell<ResourceArc<BufferResource>>,
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
    pub wireframe_desriptor_set: Option<DescriptorSetArc>,
    pub shadow_map_atlas_depth_descriptor_set: Option<DescriptorSetArc>,
}

pub enum MeshAdvDrawCall {
    Batched(MeshAdvBatchedDrawCall),
    Unbatched(MeshAdvUnbatchedDrawCall),
}

impl MeshAdvDrawCall {
    pub fn as_batched(&self) -> Option<&MeshAdvBatchedDrawCall> {
        match self {
            MeshAdvDrawCall::Batched(dc) => Some(dc),
            MeshAdvDrawCall::Unbatched(_) => None,
        }
    }

    pub fn as_unbatched(&self) -> Option<&MeshAdvUnbatchedDrawCall> {
        match self {
            MeshAdvDrawCall::Batched(_) => None,
            MeshAdvDrawCall::Unbatched(dc) => Some(dc),
        }
    }
}

pub struct MeshAdvUnbatchedDrawCall {
    pub render_object_instance_id: RenderObjectInstanceId,
    pub material_pass_resource: ResourceArc<MaterialPassResource>,
    //pub per_material_descriptor_set: Option<DescriptorSetArc>,
    pub mesh_part_index: usize,
    pub model_matrix_index: usize,
    pub material_index: Option<u32>,
    pub index_type: RafxIndexType,
    pub draw_data_index: u32,
    pub batch_index: u32,
}

pub struct MeshAdvBatchedDrawCall {
    pub batch_index: u32,
}
