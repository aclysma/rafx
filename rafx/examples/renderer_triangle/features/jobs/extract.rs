use crate::features::internal::{DemoFramePacket, DemoPerFrameData, DemoRenderFeatureTypes};
use crate::TimeState;
use rafx::assets::{AssetManagerRenderResource, MaterialAsset};
use rafx::distill::loader::handle::Handle;
use rafx::framework::render_features::ExtractJob;
use rafx::render_feature_extract_job_predule::*;
use rafx_base::resource_map::ReadBorrow;
use rafx_base::resource_ref_map::ResourceRefBorrow;
use std::sync::Arc;

pub struct DemoExtractJob<'extract> {
    time_state: ResourceRefBorrow<'extract, TimeState>,
    asset_manager: ReadBorrow<'extract, AssetManagerRenderResource>,
    triangle_material: Handle<MaterialAsset>,
}

impl<'extract> DemoExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<DemoFramePacket>,
        triangle_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                time_state: extract_context.extract_resources.fetch::<TimeState>(),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>(),
                triangle_material,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for DemoExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        context
            .frame_packet()
            .per_frame_data()
            .set(DemoPerFrameData {
                triangle_material: self
                    .asset_manager
                    .get()
                    .unwrap()
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

    type FramePacketDataT = DemoRenderFeatureTypes;
}
