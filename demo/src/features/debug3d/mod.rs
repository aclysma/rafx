use crate::features::debug3d::extract::Debug3dExtractJob;
use crate::render_contexts::{
    RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext,
};
use atelier_assets::loader::handle::Handle;
use renderer::assets::MaterialAsset;
use renderer::nodes::ExtractJob;
use renderer::nodes::RenderFeature;
use renderer::nodes::RenderFeatureIndex;
use std::convert::TryInto;

mod debug3d_resource;
mod extract;
mod prepare;
mod write;

pub use debug3d_resource::*;

pub fn create_debug3d_extract_job(
    debug3d_material: &Handle<MaterialAsset>
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(Debug3dExtractJob::new(debug3d_material))
}

pub type Debug3dUniformBufferObject = shaders::debug_vert::PerFrameUboUniform;

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct Debug3dVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

renderer::declare_render_feature!(Debug3dRenderFeature, DEBUG_3D_FEATURE_INDEX);

pub(self) struct ExtractedDebug3dData {
    line_lists: Vec<LineList3D>,
}

#[derive(Debug)]
struct Debug3dDrawCall {
    first_element: u32,
    count: u32,
}
