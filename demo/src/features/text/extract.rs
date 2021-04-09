rafx::declare_render_feature_extract_job!();

use super::{StaticResources, TextResource};
use fnv::FnvHashMap;
use rafx::assets::AssetManagerRenderResource;

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
        profiling::scope!(extract_scope);

        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        let mut text_resource = extract_context
            .extract_resources
            .fetch_mut::<TextResource>();

        let text_material = &extract_context
            .render_resources
            .fetch::<StaticResources>()
            .text_material;

        let text_material_pass = asset_manager
            .committed_asset(&text_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        let text_draw_data = text_resource.take_text_draw_data();
        let mut font_assets = FnvHashMap::default();
        for (load_handle, handle) in text_draw_data.fonts {
            let asset = asset_manager.committed_asset(&handle).unwrap().clone();
            let old = font_assets.insert(load_handle, asset);
            assert!(old.is_none());
        }

        Box::new(PrepareJobImpl::new(
            text_material_pass,
            text_draw_data.text_draw_commands,
            font_assets,
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }
}
