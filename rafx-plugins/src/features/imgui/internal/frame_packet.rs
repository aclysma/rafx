use super::*;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct ImGuiRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct ImGuiPerFrameData {
    pub imgui_draw_data: Option<ImGuiDrawData>,
    pub imgui_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub view_ubo: ImGuiUniformBufferObject,
}

impl FramePacketData for ImGuiRenderFeatureTypes {
    type PerFrameData = ImGuiPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type ImGuiFramePacket = FramePacket<ImGuiRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for ImGuiRenderFeatureTypes {
    type PerFrameSubmitData = ImGuiPerFrameSubmitData;
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = ();
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = ImGuiRenderFeature;
}

pub type ImGuiSubmitPacket = SubmitPacket<ImGuiRenderFeatureTypes>;

//-------
// WRITE
//-------

pub type ImGuiUniformBufferObject = shaders::imgui_vert::ArgsUniform;

#[derive(Default)]
pub struct ImGuiPerFrameSubmitData {
    pub vertex_buffers: Vec<ResourceArc<BufferResource>>,
    pub index_buffers: Vec<ResourceArc<BufferResource>>,
    pub per_view_descriptor_set: Option<DescriptorSetArc>,
    pub per_font_descriptor_set: Option<DescriptorSetArc>,
}
