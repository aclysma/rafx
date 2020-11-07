use crate::features::imgui::{ExtractedImGuiData, ImGuiRenderFeature, ImGuiUniformBufferObject};
use crate::render_contexts::{RenderJobExtractContext, RenderJobWriteContext, RenderJobPrepareContext};
use renderer::nodes::{
    FramePacket, RenderView, PrepareJob, RenderFeatureIndex, RenderFeature, ExtractJob,
};
use crate::features::imgui::prepare::ImGuiPrepareJobImpl;
use atelier_assets::loader::handle::Handle;
use crate::imgui_support::Sdl2ImguiManager;
use ash::vk::Extent2D;
use renderer::assets::{ImageViewResource, ResourceArc};
use renderer::assets::MaterialAsset;

// This is almost copy-pasted from glam. I wanted to avoid pulling in the entire library for a
// single function
pub fn orthographic_rh_gl(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> [[f32; 4]; 4] {
    let a = 2.0 / (right - left);
    let b = 2.0 / (top - bottom);
    let c = -2.0 / (far - near);
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -(far + near) / (far - near);

    [
        [a, 0.0, 0.0, 0.0],
        [0.0, b, 0.0, 0.0],
        [0.0, 0.0, c, 0.0],
        [tx, ty, tz, 1.0],
    ]
}

pub struct ImGuiExtractJobImpl {
    extents: Extent2D,
    imgui_material: Handle<MaterialAsset>,
    font_atlas: ResourceArc<ImageViewResource>,
}

impl ImGuiExtractJobImpl {
    pub fn new(
        extents: Extent2D,
        imgui_material: &Handle<MaterialAsset>,
        font_atlas: ResourceArc<ImageViewResource>,
    ) -> Self {
        ImGuiExtractJobImpl {
            extents,
            imgui_material: imgui_material.clone(),
            font_atlas,
        }
    }
}

impl ExtractJob<RenderJobExtractContext, RenderJobPrepareContext, RenderJobWriteContext>
    for ImGuiExtractJobImpl
{
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
    ) -> Box<dyn PrepareJob<RenderJobPrepareContext, RenderJobWriteContext>> {
        let imgui_draw_data = extract_context
            .resources
            .get::<Sdl2ImguiManager>()
            .unwrap()
            .copy_draw_data();

        let framebuffer_scale = match &imgui_draw_data {
            Some(data) => data.framebuffer_scale,
            None => [1.0, 1.0],
        };

        let view_proj = orthographic_rh_gl(
            0.0,
            self.extents.width as f32 / framebuffer_scale[0],
            0.0,
            self.extents.height as f32 / framebuffer_scale[1],
            -100.0,
            100.0,
        );

        let imgui_material_pass = extract_context
            .asset_manager
            .get_material_pass_by_index(&self.imgui_material, 0)
            .unwrap();

        let view_ubo = ImGuiUniformBufferObject { view_proj };

        Box::new(ImGuiPrepareJobImpl::new(
            ExtractedImGuiData { imgui_draw_data },
            imgui_material_pass,
            view_ubo,
            self.font_atlas,
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        ImGuiRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        ImGuiRenderFeature::feature_index()
    }
}
