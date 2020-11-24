use crate::features::imgui::extract::ImGuiExtractJobImpl;
use crate::imgui_support::ImGuiDrawData;
use crate::render_contexts::{
    RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext,
};
use ash::vk::Extent2D;
use atelier_assets::loader::handle::Handle;
use renderer::assets::MaterialAsset;
use renderer::nodes::ExtractJob;
use renderer::nodes::RenderFeature;
use renderer::nodes::RenderFeatureIndex;
use renderer::resources::{ImageViewResource, ResourceArc, VertexDataLayout, VertexDataSetLayout};
use std::convert::TryInto;

mod extract;
mod prepare;
mod write;

pub fn create_imgui_extract_job(
    extents: Extent2D,
    imgui_material: &Handle<MaterialAsset>,
    font_atlas: ResourceArc<ImageViewResource>,
) -> Box<dyn ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>> {
    Box::new(ImGuiExtractJobImpl::new(
        extents,
        imgui_material,
        font_atlas,
    ))
}

/// Per-pass "global" data
pub type ImGuiUniformBufferObject = shaders::imgui_vert::ArgsUniform;

lazy_static::lazy_static! {
    pub static ref IMGUI_VERTEX_LAYOUT : VertexDataSetLayout = {
        use renderer::resources::vk_description::Format;

        let vertex = imgui::DrawVert {
            pos: Default::default(),
            col: Default::default(),
            uv: Default::default()
        };

        VertexDataLayout::build_vertex_layout(&vertex, |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", Format::R32G32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", Format::R32G32_SFLOAT);
            builder.add_member(&vertex.col, "COLOR", Format::R8G8B8A8_UNORM);
        }).into_set()
    };
}

renderer::declare_render_feature!(ImGuiRenderFeature, DEBUG_3D_FEATURE_INDEX);

pub(self) struct ExtractedImGuiData {
    imgui_draw_data: Option<ImGuiDrawData>,
}

#[derive(Debug)]
struct ImGuiDrawCall {
    first_element: u32,
    count: u32,
}
