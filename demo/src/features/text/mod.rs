use crate::features::text::extract::TextExtractJob;
use rafx::framework::{
    BufferResource, ImageViewResource, ResourceArc, VertexDataLayout, VertexDataSetLayout,
};
use rafx::nodes::ExtractJob;
use rafx::nodes::RenderFeature;
use rafx::nodes::RenderFeatureIndex;
use std::convert::TryInto;

mod extract;
mod plugin;
mod prepare;
mod text_resource;
mod write;
pub use plugin::TextRendererPlugin;

use crate::assets::font::FontAsset;
use fnv::FnvHashMap;
use rafx::api::RafxPrimitiveTopology;
use rafx::distill::loader::LoadHandle;
pub use text_resource::*;

pub fn create_text_extract_job() -> Box<dyn ExtractJob> {
    Box::new(TextExtractJob::new())
}

pub type TextUniformBufferObject = shaders::text_vert::PerViewUboUniform;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct TextVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

lazy_static::lazy_static! {
    pub static ref TEXT_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&TextVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R32G32B32A32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

rafx::declare_render_feature!(TextRenderFeature, TEXT_FEATURE_INDEX);

pub struct TextImageUpdate {
    pub upload_buffer: ResourceArc<BufferResource>,
    pub upload_image: ResourceArc<ImageViewResource>,
}

pub(self) struct ExtractedTextData {
    text_draw_commands: Vec<TextDrawCommand>,
    font_assets: FnvHashMap<LoadHandle, FontAsset>,
}
