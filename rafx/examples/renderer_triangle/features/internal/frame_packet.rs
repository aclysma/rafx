use crate::features::DemoRenderFeature;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{MaterialPassResource, ResourceArc};

pub struct DemoRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct DemoPerFrameData {
    pub triangle_material: Option<ResourceArc<MaterialPassResource>>,
    pub seconds: f32,
}

impl FramePacketData for DemoRenderFeatureTypes {
    type PerFrameData = DemoPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type DemoFramePacket = FramePacket<DemoRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for DemoRenderFeatureTypes {
    type PerFrameSubmitData = ();
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = ();
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = DemoRenderFeature;
}

pub type DemoSubmitPacket = SubmitPacket<DemoRenderFeatureTypes>;

//-------
// WRITE
//-------
