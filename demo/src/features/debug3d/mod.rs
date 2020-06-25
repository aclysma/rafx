use crate::render_contexts::{RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext};
use atelier_assets::loader::handle::Handle;
use std::sync::atomic::{AtomicI32, Ordering};
use crate::features::debug3d::extract::Debug3dExtractJobImpl;
use renderer::vulkan::VkDeviceContext;
use renderer::resources::DescriptorSetAllocatorRef;
use renderer::resources::PipelineSwapchainInfo;
use renderer::assets::MaterialAsset;
use renderer::nodes::ExtractJob;
use renderer::nodes::RenderFeature;
use renderer::nodes::RenderFeatureIndex;
use std::convert::TryInto;

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

//
// This is boilerplate that could be macro'd
//
static DEBUG_3D_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct Debug3dRenderFeature;

impl RenderFeature for Debug3dRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        DEBUG_3D_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        DEBUG_3D_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "Debug3dRenderFeature"
    }
}

pub(self) struct ExtractedDebug3dData {
    line_lists: Vec<LineList3D>,
}

#[derive(Debug)]
struct Debug3dDrawCall {
    first_element: u32,
    count: u32,
}
