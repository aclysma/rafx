use super::*;
use glam::{Quat, Vec2, Vec3, Vec4};
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{
    BufferResource, DescriptorSetArc, ImageViewResource, MaterialPassResource, ResourceArc,
};
use std::sync::atomic::AtomicU32;

pub struct SpriteRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct SpritePerFrameData {
    pub sprite_material_pass: Option<ResourceArc<MaterialPassResource>>,
}

pub struct SpriteRenderObjectInstanceData {
    pub position: Vec3,
    pub texture_size: Vec2,
    pub scale: Vec3,
    pub rotation: Quat,
    pub color: Vec4,
    pub image_view: ResourceArc<ImageViewResource>,
}

impl FramePacketData for SpriteRenderFeatureTypes {
    type PerFrameData = SpritePerFrameData;
    type RenderObjectInstanceData = Option<SpriteRenderObjectInstanceData>;
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type SpriteFramePacket = FramePacket<SpriteRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for SpriteRenderFeatureTypes {
    type PerFrameSubmitData = ();
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = SpritePerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = SpriteDrawCall;

    type RenderFeature = SpriteRenderFeature;
}

pub type SpriteSubmitPacket = SubmitPacket<SpriteRenderFeatureTypes>;

//-------
// WRITE
//-------

#[derive(Default)]
pub struct SpritePerViewSubmitData {
    pub descriptor_set_arc: Option<DescriptorSetArc>,
    pub vertex_buffer: Option<ResourceArc<BufferResource>>,
    pub index_buffer: Option<ResourceArc<BufferResource>>,
}

pub struct SpriteDrawCall {
    pub texture_descriptor_set: Option<DescriptorSetArc>,
    pub vertex_data_offset_index: u32,
    pub index_data_offset_index: u32,
    pub index_count: AtomicU32,
}
