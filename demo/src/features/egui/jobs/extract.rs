use rafx::render_feature_extract_job_predule::*;

use super::*;
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::distill::loader::handle::Handle;
use rafx::graph::SwapchainSurfaceInfo;
use rafx::renderer::SwapchainRenderResource;
use std::marker::PhantomData;

pub struct EguiExtractJob<'extract> {
    egui_manager: EguiManager,
    swapchain_surface_info: SwapchainSurfaceInfo,
    asset_manager: AssetManagerExtractRef,
    egui_material: Handle<MaterialAsset>,
    phantom_data: PhantomData<&'extract ()>,
}

impl<'extract> EguiExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<EguiFramePacket>,
        egui_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        let egui_manager = extract_context
            .extract_resources
            .fetch_mut::<WinitEguiManager>()
            .egui_manager();

        Arc::new(ExtractJob::new(
            Self {
                egui_manager,
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
                egui_material,
                phantom_data: PhantomData,
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
        let egui_draw_data = self.egui_manager.take_draw_data();
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
