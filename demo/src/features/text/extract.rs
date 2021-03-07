use crate::features::text::plugin::TextStaticResources;
use crate::features::text::prepare::TextPrepareJobImpl;
use crate::features::text::{ExtractedTextData, TextRenderFeature, TextResource};
use fnv::FnvHashMap;
use rafx::assets::AssetManagerRenderResource;
use rafx::nodes::{
    ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex,
    RenderJobExtractContext, RenderView,
};

pub struct TextExtractJob {}

impl TextExtractJob {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob for TextExtractJob {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        _frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!("Text Extract");
        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        let mut text_resource = extract_context
            .extract_resources
            .fetch_mut::<TextResource>();

        let text_material = &extract_context
            .render_resources
            .fetch::<TextStaticResources>()
            .text_material;
        let text_material_pass = asset_manager
            .get_material_pass_by_index(&text_material, 0)
            .unwrap();

        let text_draw_data = text_resource.take_text_draw_data();
        let mut font_assets = FnvHashMap::default();
        for (load_handle, handle) in text_draw_data.fonts {
            let asset = asset_manager.committed_asset(&handle).unwrap().clone();
            let old = font_assets.insert(load_handle, asset);
            assert!(old.is_none());
        }

        Box::new(TextPrepareJobImpl::new(
            text_material_pass,
            ExtractedTextData {
                text_draw_commands: text_draw_data.text_draw_commands,
                font_assets,
            },
        ))
    }

    fn feature_debug_name(&self) -> &'static str {
        TextRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        TextRenderFeature::feature_index()
    }
}
