use crate::features::debug3d::extract::Debug3dExtractJob;
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};
use rafx::nodes::ExtractJob;
use rafx::nodes::RenderFeature;
use rafx::nodes::RenderFeatureIndex;
use std::convert::TryInto;

mod debug3d_resource;
mod extract;
mod prepare;
mod write;

pub use debug3d_resource::*;
use rafx::api::RafxPrimitiveTopology;

pub fn create_debug3d_extract_job(
) -> Box<dyn ExtractJob> {
    Box::new(Debug3dExtractJob::new())
}

pub type Debug3dUniformBufferObject = shaders::debug_vert::PerFrameUboUniform;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct Debug3dVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

lazy_static::lazy_static! {
    pub static ref DEBUG_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&Debug3dVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R32G32B32A32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::LineStrip)
    };
}

rafx::declare_render_feature!(Debug3dRenderFeature, DEBUG_3D_FEATURE_INDEX);

pub(self) struct ExtractedDebug3dData {
    line_lists: Vec<LineList3D>,
}

#[derive(Debug)]
struct Debug3dDrawCall {
    first_element: u32,
    count: u32,
}
