use rafx::render_feature_extract_job_predule::*;

use super::*;
use rafx::assets::{AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_map::ReadBorrow;
use rafx::base::resource_ref_map::ResourceRefBorrowMut;
use rafx::distill::loader::handle::Handle;
use rafx::graph::SwapchainSurfaceInfo;

pub struct EguiExtractJob<'extract> {
    sdl2_egui_manager: TrustCell<ResourceRefBorrowMut<'extract, Sdl2EguiManager>>,
    swapchain_surface_info: ReadBorrow<'extract, SwapchainSurfaceInfo>,
    asset_manager: ReadBorrow<'extract, AssetManagerRenderResource>,
    egui_material: Handle<MaterialAsset>,
}

impl<'extract> EguiExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<EguiFramePacket>,
        egui_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                sdl2_egui_manager: TrustCell::new(
                    extract_context
                        .extract_resources
                        .fetch_mut::<Sdl2EguiManager>(),
                ),
                swapchain_surface_info: extract_context
                    .render_resources
                    .fetch::<SwapchainSurfaceInfo>(),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>(),
                egui_material,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for EguiExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        let sdl2_egui_manager = &mut self.sdl2_egui_manager.borrow_mut();
        let egui_draw_data = sdl2_egui_manager.egui_manager().take_draw_data();
        let view_ubo = {
            let pixels_per_point = match &egui_draw_data {
                Some(data) => data.pixels_per_point,
                None => 1.0,
            };

            let top = 0.0;
            let bottom = self.swapchain_surface_info.extents.height as f32 / pixels_per_point;

            let view_proj = glam::Mat4::orthographic_rh(
                0.0,
                self.swapchain_surface_info.extents.width as f32 / pixels_per_point,
                bottom,
                top,
                -100.0,
                100.0,
            );

            EguiUniformBufferObject {
                mvp: view_proj.to_cols_array_2d(),
            }
        };

        context
            .frame_packet()
            .per_frame_data()
            .set(EguiPerFrameData {
                egui_draw_data,
                egui_material_pass: self
                    .asset_manager
                    .committed_asset(&self.egui_material)
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

    type FramePacketDataT = EguiRenderFeatureTypes;
}
