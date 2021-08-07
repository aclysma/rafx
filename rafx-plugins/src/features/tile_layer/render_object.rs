use super::TileLayerRenderFeature;
use crate::assets::ldtk::LdtkLayerDrawCallData;
use rafx::framework::{BufferResource, DescriptorSetArc, ResourceArc};
use rafx::render_features::RenderObjectSet;

#[derive(Clone)]
pub struct TileLayerRenderObject {
    pub per_layer_descriptor_set: DescriptorSetArc,
    pub draw_call_data: Vec<LdtkLayerDrawCallData>,
    pub vertex_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
    pub z_position: f32,
}

pub type TileLayerRenderObjectSet = RenderObjectSet<TileLayerRenderFeature, TileLayerRenderObject>;
