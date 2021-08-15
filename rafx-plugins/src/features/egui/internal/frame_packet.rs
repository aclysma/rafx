use super::*;
use crate::shaders;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct EguiRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct EguiPerFrameData {
    pub egui_draw_data: Option<EguiDrawData>,
    pub egui_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub view_ubo: EguiUniformBufferObject,
}

impl FramePacketData for EguiRenderFeatureTypes {
    type PerFrameData = EguiPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type EguiFramePacket = FramePacket<EguiRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for EguiRenderFeatureTypes {
    type PerFrameSubmitData = EguiPerFrameSubmitData;
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = ();
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = EguiRenderFeature;
}

pub type EguiSubmitPacket = SubmitPacket<EguiRenderFeatureTypes>;

//-------
// WRITE
//-------

pub type EguiUniformBufferObject = shaders::egui_vert::ArgsUniform;

#[derive(Default)]
pub struct EguiPerFrameSubmitData {
    pub vertex_buffer: Option<ResourceArc<BufferResource>>,
    pub index_buffer: Option<ResourceArc<BufferResource>>,
    pub per_view_descriptor_set: Option<DescriptorSetArc>,
    pub per_font_descriptor_set: Option<DescriptorSetArc>,
    pub image_update: Option<EguiImageUpdate>,
}
