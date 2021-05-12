use super::*;
use rafx::framework::render_features::render_features_prelude::*;
use rafx::framework::{DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct TileLayerRenderFeatureTypes;

//---------
// EXTRACT
//---------

pub type TileLayerRenderObjectStaticData = TileLayerRenderObject;

pub struct TileLayerPerFrameData {
    pub tile_layer_material_pass: Option<ResourceArc<MaterialPassResource>>,
}

impl FramePacketData for TileLayerRenderFeatureTypes {
    type PerFrameData = TileLayerPerFrameData;
    type RenderObjectInstanceData = ();
    type PerViewData = ();
    type RenderObjectInstancePerViewData = ();
}

pub type TileLayerFramePacket = FramePacket<TileLayerRenderFeatureTypes>;
pub type TileLayerViewPacket = ViewPacket<TileLayerRenderFeatureTypes>;

//---------
// PREPARE
//---------

impl SubmitPacketData for TileLayerRenderFeatureTypes {
    type PerFrameSubmitData = ();
    type RenderObjectInstanceSubmitData = ();
    type PerViewSubmitData = TileLayerPerViewSubmitData;
    type RenderObjectInstancePerViewSubmitData = ();
    type SubmitNodeData = TileLayerSubmitNodeData;

    type RenderFeature = TileLayerRenderFeature;
}

pub type TileLayerSubmitPacket = SubmitPacket<TileLayerRenderFeatureTypes>;
pub type TileLayerViewSubmitPacket = ViewSubmitPacket<TileLayerRenderFeatureTypes>;

//-------
// WRITE
//-------

pub type TileLayerUniformBufferObject = shaders::tile_layer_vert::ArgsUniform;

pub struct TileLayerPerViewSubmitData {
    pub descriptor_set_arc: Option<DescriptorSetArc>,
}

pub struct TileLayerSubmitNodeData {
    pub render_object_id: RenderObjectId,
}
