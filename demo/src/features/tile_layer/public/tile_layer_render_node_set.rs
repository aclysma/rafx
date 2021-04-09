use super::super::render_feature_index;
use crate::assets::ldtk::LdtkLayerDrawCallData;
use rafx::base::slab::{DropSlab, DropSlabKey};
use rafx::framework::{BufferResource, DescriptorSetArc, ResourceArc};
use rafx::nodes::{GenericRenderNodeHandle, RenderFeatureIndex, RenderNodeCount, RenderNodeSet};

//
// This is boiler-platish
//
#[derive(Clone)]
pub struct TileLayerRenderNode {
    pub per_layer_descriptor_set: DescriptorSetArc,
    pub draw_call_data: Vec<LdtkLayerDrawCallData>,
    pub vertex_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
    pub z_position: f32,
}

#[derive(Clone)]
pub struct TileLayerRenderNodeHandle(pub DropSlabKey<TileLayerRenderNode>);

impl TileLayerRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(render_feature_index(), self.0.index())
    }
}

impl Into<GenericRenderNodeHandle> for TileLayerRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct TileLayerRenderNodeSet {
    pub(in crate::features::tile_layer) tile_layers: DropSlab<TileLayerRenderNode>,
}

impl TileLayerRenderNodeSet {
    pub fn register_tile_layer(
        &mut self,
        node: TileLayerRenderNode,
    ) -> TileLayerRenderNodeHandle {
        TileLayerRenderNodeHandle(self.tile_layers.allocate(node))
    }

    pub fn get_mut(
        &mut self,
        handle: &TileLayerRenderNodeHandle,
    ) -> Option<&mut TileLayerRenderNode> {
        self.tile_layers.get_mut(&handle.0)
    }

    pub fn update(&mut self) {
        self.tile_layers.process_drops();
    }
}

impl RenderNodeSet for TileLayerRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.tile_layers.storage_size() as RenderNodeCount
    }
}
