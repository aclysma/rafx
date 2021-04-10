use rafx::render_feature_extract_job_predule::*;

use super::{SkyboxPrepareJob, SkyboxStaticResources};
use rafx::assets::AssetManagerRenderResource;

pub struct SkyboxExtractJob {}

impl SkyboxExtractJob {
    pub fn new() -> Self {
        SkyboxExtractJob {}
    }
}

impl ExtractJob for SkyboxExtractJob {
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

        let static_resources = extract_context
            .render_resources
            .fetch::<SkyboxStaticResources>();

        let skybox_material = asset_manager
            .committed_asset(&static_resources.skybox_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        let skybox_texture = asset_manager
            .committed_asset(&static_resources.skybox_texture)
            .unwrap()
            .image_view
            .clone();

        Box::new(SkyboxPrepareJob::new(skybox_material, skybox_texture))
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
