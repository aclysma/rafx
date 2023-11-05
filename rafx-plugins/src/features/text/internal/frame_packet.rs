use super::*;
use crate::assets::font::FontAsset;
use crate::shaders::text::text_vert;
use fnv::FnvHashMap;
use hydrate_base::LoadHandle;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct TextRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct TextPerFrameData {
    pub text_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub text_draw_commands: Vec<TextDrawCommand>,
    pub font_assets: FnvHashMap<LoadHandle, FontAsset>,
}

impl FramePacketData for TextRenderFeatureTypes {
    type PerFrameData = TextPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type TextFramePacket = FramePacket<TextRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for TextRenderFeatureTypes {
    type PerFrameSubmitData = TextPerFrameSubmitData;
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = TextPerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = TextRenderFeature;
}

pub type TextSubmitPacket = SubmitPacket<TextRenderFeatureTypes>;

//-------
// WRITE
//-------

pub type TextUniformBufferObject = text_vert::PerViewUboUniform;

#[derive(Default)]
pub struct TextPerFrameSubmitData {
    pub draw_call_buffers: Vec<TextDrawCallBuffers>,
    pub draw_call_metas: Vec<TextDrawCallMeta>,
    pub per_font_descriptor_sets: Vec<DescriptorSetArc>,
    pub image_updates: Vec<TextImageUpdate>,
}

pub struct TextDrawCallMeta {
    pub font_descriptor_index: u32,
    pub buffer_index: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub z_position: f32,
}

pub struct TextDrawCallBuffers {
    pub vertex_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
}

pub struct TextPerViewSubmitData {
    pub descriptor_set_arc: Option<DescriptorSetArc>,
}
