use rafx::render_feature_extract_job_predule::*;

use super::*;
use rafx::assets::{AssetManagerRenderResource, ImageAsset, MaterialAsset};
use rafx::base::resource_map::ReadBorrow;
use rafx::distill::loader::handle::Handle;
use std::sync::Arc;

pub struct SkyboxExtractJob<'extract> {
    asset_manager: ReadBorrow<'extract, AssetManagerRenderResource>,
    skybox_material: Handle<MaterialAsset>,
    skybox_texture: Handle<ImageAsset>,
}

impl<'extract> SkyboxExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<SkyboxFramePacket>,
        skybox_material: Handle<MaterialAsset>,
        skybox_texture: Handle<ImageAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>(),
                skybox_material,
                skybox_texture,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for SkyboxExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        context
            .frame_packet()
            .per_frame_data()
            .set(SkyboxPerFrameData {
                skybox_material_pass: self
                    .asset_manager
                    .get()
                    .unwrap()
                    .committed_asset(&self.skybox_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
                skybox_texture: self
                    .asset_manager
                    .get()
                    .unwrap()
                    .committed_asset(&self.skybox_texture)
                    .and_then(|x| Some(x.image_view.clone())),
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

    type FramePacketDataT = SkyboxRenderFeatureTypes;
}
