use super::*;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{DescriptorSetArc, ImageViewResource, MaterialPassResource, ResourceArc};

pub struct SkyboxRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub struct SkyboxPerFrameData {
    pub skybox_material_pass: Option<ResourceArc<MaterialPassResource>>,
    pub skybox_texture: Option<ResourceArc<ImageViewResource>>,
}

impl FramePacketData for SkyboxRenderFeatureTypes {
    type PerFrameData = SkyboxPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type SkyboxFramePacket = FramePacket<SkyboxRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for SkyboxRenderFeatureTypes {
    type PerFrameSubmitData = ();
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = SkyboxPerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = ();

    type RenderFeature = SkyboxRenderFeature;
}

pub type SkyboxSubmitPacket = SubmitPacket<SkyboxRenderFeatureTypes>;

//-------
// WRITE
//-------

pub struct SkyboxPerViewSubmitData {
    pub descriptor_set_arc: Option<DescriptorSetArc>,
}
