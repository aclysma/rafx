use rafx::render_feature_extract_job_predule::*;

use super::*;
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_map::ReadBorrow;
use rafx::base::resource_ref_map::ResourceRefBorrowMut;
use rafx::distill::loader::handle::Handle;
use rafx::graph::SwapchainSurfaceInfo;
use rafx::renderer::SwapchainRenderResource;

pub struct ImGuiExtractJob<'extract> {
    sdl2_imgui_manager: TrustCell<ResourceRefBorrowMut<'extract, Sdl2ImguiManager>>,
    swapchain_surface_info: SwapchainSurfaceInfo,
    asset_manager: AssetManagerExtractRef,
    imgui_material: Handle<MaterialAsset>,
}

impl<'extract> ImGuiExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<ImGuiFramePacket>,
        imgui_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                sdl2_imgui_manager: TrustCell::new(
                    extract_context
                        .extract_resources
                        .fetch_mut::<Sdl2ImguiManager>(),
                ),
                swapchain_surface_info: extract_context
                    .render_resources
                    .fetch::<SwapchainRenderResource>()
                    .get()
                    .swapchain_surface_info
                    .clone(),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                imgui_material,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for ImGuiExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        let sdl2_imgui_manager_mut = &mut self.sdl2_imgui_manager.borrow_mut();
        let imgui_draw_data = sdl2_imgui_manager_mut.copy_draw_data();
        let view_ubo = {
            let framebuffer_scale = match &imgui_draw_data {
                Some(data) => data.framebuffer_scale,
                None => [1.0, 1.0],
            };

            let top = 0.0;
            let bottom = self.swapchain_surface_info.extents.height as f32 / framebuffer_scale[1];

            let view_proj = glam::Mat4::orthographic_rh(
                0.0,
                self.swapchain_surface_info.extents.width as f32 / framebuffer_scale[0],
                bottom,
                top,
                -100.0,
                100.0,
            );

            ImGuiUniformBufferObject {
                mvp: view_proj.to_cols_array_2d(),
            }
        };

        context
            .frame_packet()
            .per_frame_data()
            .set(ImGuiPerFrameData {
                imgui_draw_data,
                imgui_material_pass: self
                    .asset_manager
                    .committed_asset(&self.imgui_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
                view_ubo,
            });
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = ImGuiRenderFeatureTypes;
}
