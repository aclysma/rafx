use crate::render_contexts::{RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext};
use atelier_assets::loader::handle::Handle;
use std::sync::atomic::{AtomicI32, Ordering};
use crate::features::imgui::extract::ImGuiExtractJobImpl;
use renderer::vulkan::VkDeviceContext;
use renderer::resources::DescriptorSetAllocatorRef;
use renderer::resources::PipelineSwapchainInfo;
use renderer::assets::MaterialAsset;
use renderer::nodes::ExtractJob;
use renderer::nodes::RenderFeature;
use renderer::nodes::RenderFeatureIndex;
use std::convert::TryInto;
use crate::imgui_support::ImGuiDrawData;
use ash::vk::Extent2D;
use renderer::resources::{ImageViewResource, ResourceArc};

mod extract;
mod prepare;
mod write;

pub fn create_imgui_extract_job(
    device_context: VkDeviceContext,
    descriptor_set_allocator: DescriptorSetAllocatorRef,
    pipeline_info: PipelineSwapchainInfo,
    extents: Extent2D,
    imgui_material: &Handle<MaterialAsset>,
    font_atlas: ResourceArc<ImageViewResource>,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(ImGuiExtractJobImpl::new(
        device_context,
        descriptor_set_allocator,
        pipeline_info,
        extents,
        imgui_material,
        font_atlas,
    ))
}

/// Per-pass "global" data
#[derive(Clone, Debug, Copy)]
struct ImGuiUniformBufferObject {
    // View and projection matrices
    view_proj: [[f32; 4]; 4],
}

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct ImGuiVertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

//
// This is boilerplate that could be macro'd
//
static DEBUG_3D_FEATURE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct ImGuiRenderFeature;

impl RenderFeature for ImGuiRenderFeature {
    fn set_feature_index(index: RenderFeatureIndex) {
        DEBUG_3D_FEATURE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn feature_index() -> RenderFeatureIndex {
        DEBUG_3D_FEATURE_INDEX.load(Ordering::Acquire) as RenderFeatureIndex
    }

    fn feature_debug_name() -> &'static str {
        "ImGuiRenderFeature"
    }
}

pub(self) struct ExtractedImGuiData {
    imgui_draw_data: Option<ImGuiDrawData>,
}

#[derive(Debug)]
struct ImGuiDrawCall {
    first_element: u32,
    count: u32,
}
