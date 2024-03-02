use rafx::render_feature_extract_job_predule::*;

use super::*;
use hydrate_base::handle::Handle;
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_ref_map::ResourceRefBorrow;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct SkyboxExtractJob<'extract> {
    asset_manager: AssetManagerExtractRef,
    skybox_material: Handle<MaterialAsset>,
    skybox_resource: ResourceRefBorrow<'extract, SkyboxResource>,
    phantom_data: PhantomData<&'extract ()>,
}

impl<'extract> SkyboxExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<SkyboxFramePacket>,
        skybox_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                skybox_resource: extract_context.extract_resources.fetch::<SkyboxResource>(),
                skybox_material,
                phantom_data: PhantomData,
            },
            extract_context,
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for SkyboxExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        let skybox_texture = self
            .skybox_resource
            .skybox_texture
            .as_ref()
            .map(|x| {
                self.asset_manager
                    .committed_asset(x)
                    .map(|x| x.image_view.clone())
            })
            .flatten();

        context
            .frame_packet()
            .per_frame_data()
            .set(SkyboxPerFrameData {
                skybox_material_pass: self
                    .asset_manager
                    .committed_asset(&self.skybox_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
                skybox_texture,
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
