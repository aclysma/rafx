use distill::loader::handle::Handle;
use rafx::assets::ImageAsset;
use rafx::base::slab::{DropSlab, DropSlabKey};
use rafx::nodes::{
    ExtractJob, GenericRenderNodeHandle, RenderFeature, RenderFeatureIndex, RenderNodeCount,
    RenderNodeSet,
};
use std::convert::TryInto;

mod extract;
use extract::SpriteExtractJob;

mod prepare;

mod write;

mod plugin;
pub use plugin::SpriteRendererPlugin;

use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{
    DescriptorSetArc, ImageViewResource, ResourceArc, VertexDataLayout, VertexDataSetLayout,
};
use write::SpriteCommandWriter;

/// Per-pass "global" data
pub type SpriteUniformBufferObject = shaders::sprite_vert::ArgsUniform;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct SpriteVertex {
    pub pos: [f32; 2],
    pub tex_coord: [f32; 2],
    //color: [u8; 4],
}

lazy_static::lazy_static! {
    pub static ref SPRITE_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&SpriteVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.tex_coord, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

/// Used as static data to represent a quad
#[derive(Clone, Debug, Copy)]
struct QuadVertex {
    pos: [f32; 3],
    tex_coord: [f32; 2],
}

/// Static data the represents a "unit" quad
const QUAD_VERTEX_LIST: [QuadVertex; 4] = [
    // Top Right
    QuadVertex {
        pos: [0.5, 0.5, 0.0],
        tex_coord: [1.0, 0.0],
    },
    // Top Left
    QuadVertex {
        pos: [-0.5, 0.5, 0.0],
        tex_coord: [0.0, 0.0],
    },
    // Bottom Right
    QuadVertex {
        pos: [0.5, -0.5, 0.0],
        tex_coord: [1.0, 1.0],
    },
    // Bottom Left
    QuadVertex {
        pos: [-0.5, -0.5, 0.0],
        tex_coord: [0.0, 1.0],
    },
];

/// Draw order of QUAD_VERTEX_LIST
const QUAD_INDEX_LIST: [u16; 6] = [0, 1, 2, 2, 1, 3];

pub fn create_sprite_extract_job() -> Box<dyn ExtractJob> {
    Box::new(SpriteExtractJob::new())
}

//
// This is boiler-platish
//
pub struct SpriteRenderNode {
    pub position: glam::Vec3,
    pub alpha: f32,
    pub image: Handle<ImageAsset>,
}

#[derive(Clone)]
pub struct SpriteRenderNodeHandle(pub DropSlabKey<SpriteRenderNode>);

impl SpriteRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(
            <SpriteRenderFeature as RenderFeature>::feature_index(),
            self.0.index(),
        )
    }
}

impl Into<GenericRenderNodeHandle> for SpriteRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct SpriteRenderNodeSet {
    sprites: DropSlab<SpriteRenderNode>,
}

impl SpriteRenderNodeSet {
    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn get_mut(
        &mut self,
        handle: &SpriteRenderNodeHandle,
    ) -> Option<&mut SpriteRenderNode> {
        self.sprites.get_mut(&handle.0)
    }

    pub fn update(&mut self) {
        self.sprites.process_drops();
    }
}

impl RenderNodeSet for SpriteRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.sprites.storage_size() as RenderNodeCount
    }
}

rafx::declare_render_feature!(SpriteRenderFeature, SPRITE_FEATURE_INDEX);

#[derive(Debug)]
pub(self) struct ExtractedSpriteData {
    position: glam::Vec3,
    texture_size: glam::Vec2,
    scale: f32,
    rotation: f32,
    alpha: f32,
    image_view: ResourceArc<ImageViewResource>,
}

#[derive(Debug)]
pub struct SpriteDrawCall {
    index_buffer_first_element: u16,
    index_buffer_count: u16,
    texture_descriptor_set: DescriptorSetArc,
}
