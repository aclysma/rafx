use rafx::render_feature_extract_job_predule::*;

use super::prepare::{ImGuiUniformBufferObject, PrepareJobImpl};
use super::Sdl2ImguiManager;
use super::StaticResources;
use rafx::assets::AssetManagerRenderResource;
use rafx::graph::SwapchainSurfaceInfo;

pub struct ExtractJobImpl {}

impl ExtractJobImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob for ExtractJobImpl {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!(super::extract_scope);

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
            .fetch::<StaticResources>()
            .imgui_material;
        let imgui_material_pass = asset_manager
            .committed_asset(imgui_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        let static_resources = &extract_context.render_resources.fetch::<StaticResources>();
        let view_ubo = ImGuiUniformBufferObject {
            mvp: view_proj.to_cols_array_2d(),
        };

        Box::new(PrepareJobImpl::new(
            imgui_draw_data,
            imgui_material_pass,
            view_ubo,
            static_resources.imgui_font_atlas_image_view.clone(),
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
