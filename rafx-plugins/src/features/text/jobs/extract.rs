use rafx::render_feature_extract_job_predule::*;

use super::*;
use fnv::FnvHashMap;
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_ref_map::ResourceRefBorrowMut;
use rafx::distill::loader::handle::Handle;

pub struct TextExtractJob<'extract> {
    text_resource: TrustCell<ResourceRefBorrowMut<'extract, TextResource>>,
    asset_manager: AssetManagerExtractRef,
    text_material: Handle<MaterialAsset>,
}

impl<'extract> TextExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<TextFramePacket>,
        text_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                text_resource: TrustCell::new(
                    extract_context
                        .extract_resources
                        .fetch_mut::<TextResource>(),
                ),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                text_material,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for TextExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        let mut font_assets = FnvHashMap::default();
        let text_resource = &mut self.text_resource.borrow_mut();
        let text_draw_data = text_resource.take_text_draw_data();
        for (load_handle, handle) in text_draw_data.fonts {
            let asset = self.asset_manager.committed_asset(&handle).unwrap().clone();
            let old = font_assets.insert(load_handle, asset);
            assert!(old.is_none());
        }

        context
            .frame_packet()
            .per_frame_data()
            .set(TextPerFrameData {
                text_material_pass: self
                    .asset_manager
                    .committed_asset(&self.text_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
                text_draw_commands: text_draw_data.text_draw_commands,
                font_assets,
            })
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = TextRenderFeatureTypes;
}
