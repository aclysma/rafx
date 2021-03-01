use crate::features::imgui::{ExtractedImGuiData, ImGuiRenderFeature, ImGuiUniformBufferObject};
use crate::imgui_support::Sdl2ImguiManager;
use crate::{
    features::imgui::prepare::ImGuiPrepareJobImpl,
    game_renderer::{GameRendererStaticResources, ImguiFontAtlas},
};
use rafx::graph::SwapchainSurfaceInfo;
use rafx::nodes::{ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex, RenderView, RenderJobExtractContext};
use crate::legion_support::LegionResources;
use rafx::assets::AssetManagerRenderResource;

pub struct ImGuiExtractJobImpl {}

impl ImGuiExtractJobImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob
    for ImGuiExtractJobImpl
{
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[&RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!("ImGui Extract");
        let legion_resources = extract_context.render_resources.fetch::<LegionResources>();
        let asset_manager = extract_context.render_resources.fetch::<AssetManagerRenderResource>();
        let imgui_draw_data = legion_resources
            .get::<Sdl2ImguiManager>()
            .unwrap()
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
            .fetch::<GameRendererStaticResources>()
            .imgui_material;
        let imgui_material_pass = asset_manager
            .get_material_pass_by_index(imgui_material, 0)
            .unwrap();

        let font_atlas = &extract_context.render_resources.fetch::<ImguiFontAtlas>().0;
        let view_ubo = ImGuiUniformBufferObject {
            mvp: view_proj.to_cols_array_2d(),
        };

        Box::new(ImGuiPrepareJobImpl::new(
            ExtractedImGuiData { imgui_draw_data },
            imgui_material_pass,
            view_ubo,
            font_atlas.clone(),
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        ImGuiRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        ImGuiRenderFeature::feature_index()
    }
}
