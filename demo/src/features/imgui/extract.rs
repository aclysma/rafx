use super::Sdl2ImguiManager;
use crate::features::imgui::plugin::ImguiStaticResources;
use crate::features::imgui::prepare::ImGuiPrepareJobImpl;
use crate::features::imgui::{ExtractedImGuiData, ImGuiRenderFeature, ImGuiUniformBufferObject};
use rafx::assets::AssetManagerRenderResource;
use rafx::graph::SwapchainSurfaceInfo;
use rafx::nodes::{
    ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex,
    RenderJobExtractContext, RenderView,
};

pub struct ImGuiExtractJobImpl {}

impl ImGuiExtractJobImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob for ImGuiExtractJobImpl {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!("ImGui Extract");
        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();
        let imgui_draw_data = extract_context
            .extract_resources
            .fetch::<Sdl2ImguiManager>()
            .copy_draw_data();

        let framebuffer_scale = match &imgui_draw_data {
            Some(data) => data.framebuffer_scale,
            None => [1.0, 1.0],
        };

        let swapchain_info = extract_context
            .render_resources
            .fetch::<SwapchainSurfaceInfo>();

        let top = 0.0;
        let bottom = swapchain_info.extents.height as f32 / framebuffer_scale[1];

        let view_proj = glam::Mat4::orthographic_rh(
            0.0,
            swapchain_info.extents.width as f32 / framebuffer_scale[0],
            bottom,
            top,
            -100.0,
            100.0,
        );

        let imgui_material = &extract_context
            .render_resources
            .fetch::<ImguiStaticResources>()
            .imgui_material;
        let imgui_material_pass = asset_manager
            .get_material_pass_by_index(imgui_material, 0)
            .unwrap();

        let static_resources = &extract_context
            .render_resources
            .fetch::<ImguiStaticResources>();
        let view_ubo = ImGuiUniformBufferObject {
            mvp: view_proj.to_cols_array_2d(),
        };

        Box::new(ImGuiPrepareJobImpl::new(
            ExtractedImGuiData { imgui_draw_data },
            imgui_material_pass,
            view_ubo,
            static_resources.imgui_font_atlas_image_view.clone(),
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        ImGuiRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        ImGuiRenderFeature::feature_index()
    }
}
