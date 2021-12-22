use super::*;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{MaterialPassResource, ResourceArc};

pub struct DebugPipRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct DebugPipPerFrameData {
    pub debug_pip_material_pass: Option<ResourceArc<MaterialPassResource>>,
}

impl FramePacketData for DebugPipRenderFeatureTypes {
    type PerFrameData = DebugPipPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type DebugPipFramePacket = FramePacket<DebugPipRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for DebugPipRenderFeatureTypes {
    type PerFrameSubmitData = ();
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = DebugPipPerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = DebugPipRenderFeature;
}

pub type DebugPipSubmitPacket = SubmitPacket<DebugPipRenderFeatureTypes>;

//-------
// WRITE
//-------

pub struct DebugPipPerViewSubmitData {}
