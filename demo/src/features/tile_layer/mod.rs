use rafx::base::slab::{DropSlab, DropSlabKey};
use rafx::nodes::{
    ExtractJob, GenericRenderNodeHandle, RenderFeature, RenderFeatureIndex, RenderNodeCount,
    RenderNodeSet,
};
use std::convert::TryInto;

mod extract;
use extract::TileLayerExtractJob;

mod prepare;

mod write;

mod plugin;
pub use plugin::TileLayerRendererPlugin;

mod tile_layer_resource;
pub use tile_layer_resource::TileLayerResource;

use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{DescriptorSetArc, ResourceArc, VertexDataLayout, VertexDataSetLayout, BufferResource};
use write::TileLayerCommandWriter;
use crate::assets::ldtk::LdtkLayerDrawCallData;

/// Per-pass "global" data
pub type TileLayerUniformBufferObject = shaders::tile_layer_vert::ArgsUniform;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct TileLayerVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    //color: [u8; 4],
}

lazy_static::lazy_static! {
    pub static ref TILE_LAYER_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&TileLayerVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

pub fn create_tile_layer_extract_job() -> Box<dyn ExtractJob> {
    Box::new(TileLayerExtractJob::new())
}

//
// This is boiler-platish
//
#[derive(Clone)]
pub struct TileLayerRenderNode {
    per_layer_descriptor_set: DescriptorSetArc,
    draw_call_data: Vec<LdtkLayerDrawCallData>,
    vertex_buffer: ResourceArc<BufferResource>,
    index_buffer: ResourceArc<BufferResource>,
    z_position: f32,
}

#[derive(Clone)]
pub struct TileLayerRenderNodeHandle(pub DropSlabKey<TileLayerRenderNode>);

impl TileLayerRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <TileLayerRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

impl Into<GenericRenderNodeHandle> for TileLayerRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct TileLayerRenderNodeSet {
    tile_layers: DropSlab<TileLayerRenderNode>,
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
        TileLayerRenderFeature::feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.tile_layers.storage_size() as RenderNodeCount
    }
}

rafx::declare_render_feature!(TileLayerRenderFeature, TILE_LAYER_FEATURE_INDEX);

// #[derive(Debug)]
// pub(self) struct ExtractedTileLayerData {
//     position: glam::Vec3,
//     texture_size: glam::Vec2,
//     scale: f32,
//     rotation: f32,
//     alpha: f32,
//     image_view: ResourceArc<ImageViewResource>,
// }
//
// #[derive(Debug)]
// pub struct TileLayerDrawCall {
//     index_buffer_first_element: u16,
//     index_buffer_count: u16,
//     texture_descriptor_set: DescriptorSetArc,
// }
