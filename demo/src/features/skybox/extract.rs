rafx::declare_render_feature_extract_job!();

use super::StaticResources;
use rafx::assets::AssetManagerRenderResource;

pub struct ExtractJobImpl {}

impl ExtractJobImpl {
    pub fn new() -> Self {
        ExtractJobImpl {}
    }
}

impl ExtractJob for ExtractJobImpl {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!(extract_scope);

        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        let static_resources = extract_context.render_resources.fetch::<StaticResources>();

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

        Box::new(PrepareJobImpl::new(skybox_material, skybox_texture))
    }

    fn feature_debug_name(&self) -> &'static str {
        render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }
}
