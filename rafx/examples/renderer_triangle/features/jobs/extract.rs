use crate::features::internal::{
    ExampleFramePacket, ExamplePerFrameData, ExampleRenderFeatureTypes,
};
use crate::TimeState;
use hydrate_base::handle::Handle;
use rafx::assets::{AssetManagerRenderResource, MaterialAsset};
use rafx::framework::render_features::ExtractJob;
use rafx::render_feature_extract_job_predule::*;
use rafx_assets::AssetManagerExtractRef;
use rafx_base::resource_ref_map::ResourceRefBorrow;
use std::sync::Arc;

pub struct ExampleExtractJob<'extract> {
    time_state: ResourceRefBorrow<'extract, TimeState>,
    asset_manager: AssetManagerExtractRef,
    triangle_material: Handle<MaterialAsset>,
}

impl<'extract> ExampleExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<ExampleFramePacket>,
        triangle_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                time_state: extract_context.extract_resources.fetch::<TimeState>(),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                triangle_material,
            },
            extract_context,
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for ExampleExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        context
            .frame_packet()
            .per_frame_data()
            .set(ExamplePerFrameData {
                triangle_material: self
                    .asset_manager
                    .committed_asset(&self.triangle_material)
                    .and_then(|x| x.get_single_material_pass().ok()),
                seconds: self.time_state.total_time().as_secs_f32(),
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

    type FramePacketDataT = ExampleRenderFeatureTypes;
}
