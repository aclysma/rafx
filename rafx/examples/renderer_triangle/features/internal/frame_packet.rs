use crate::features::ExampleRenderFeature;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{MaterialPassResource, ResourceArc};

pub struct ExampleRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct ExamplePerFrameData {
    pub triangle_material: Option<ResourceArc<MaterialPassResource>>,
    pub seconds: f32,
}

impl FramePacketData for ExampleRenderFeatureTypes {
    type PerFrameData = ExamplePerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type ExampleFramePacket = FramePacket<ExampleRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for ExampleRenderFeatureTypes {
    type PerFrameSubmitData = ();
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = ();
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = ExampleRenderFeature;
}

pub type ExampleSubmitPacket = SubmitPacket<ExampleRenderFeatureTypes>;

//-------
// WRITE
//-------
