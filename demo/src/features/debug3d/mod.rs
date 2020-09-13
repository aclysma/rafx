use crate::render_contexts::{RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext};
use atelier_assets::loader::handle::Handle;
use crate::features::debug3d::extract::Debug3dExtractJobImpl;
use renderer::vulkan::VkDeviceContext;
use renderer::assets::DescriptorSetAllocatorRef;
use renderer::assets::PipelineSwapchainInfo;
use renderer::nodes::ExtractJob;
use renderer::nodes::RenderFeature;
use renderer::nodes::RenderFeatureIndex;
use std::convert::TryInto;
use renderer::assets::MaterialAsset;

mod extract;
mod prepare;
mod write;
mod debug3d_resource;

pub use debug3d_resource::*;

pub fn create_debug3d_extract_job(
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    debug3d_material: &Handle<MaterialAsset>,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(Debug3dExtractJobImpl::new(
        device_context,
        descriptor_set_allocator,
        pipeline_info,
        debug3d_material,
    ))
}

/// Per-pass "global" data
#[derive(Clone, Debug, Copy)]
struct Debug3dUniformBufferObject {
    // View and projection matrices
    view_proj: [[f32; 4]; 4],
}

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
