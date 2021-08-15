use super::*;
use crate::shaders;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct Debug3DRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct Debug3DPerFrameData {
    pub debug3d_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub line_lists: Vec<LineList3D>,
}

impl FramePacketData for Debug3DRenderFeatureTypes {
    type PerFrameData = Debug3DPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

#[allow(dead_code)]
pub type Debug3DFramePacket = FramePacket<Debug3DRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for Debug3DRenderFeatureTypes {
    type PerFrameSubmitData = Debug3DPerFrameSubmitData;
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = Debug3DPerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = Debug3DRenderFeature;
}

pub type Debug3DSubmitPacket = SubmitPacket<Debug3DRenderFeatureTypes>;

//-------
// WRITE
//-------

pub type Debug3DUniformBufferObject = shaders::debug_vert::PerFrameUboUniform;

#[derive(Default)]
pub struct Debug3DPerFrameSubmitData {
    pub vertex_buffer: Option<ResourceArc<BufferResource>>,
    pub vertex_list: Vec<Debug3DVertex>,
    pub draw_calls: Vec<Debug3DDrawCall>,
}

pub struct Debug3DPerViewSubmitData {
    pub descriptor_set_arc: Option<DescriptorSetArc>,
}

pub struct Debug3DDrawCall {
    pub first_element: u32,
    pub count: u32,
}
