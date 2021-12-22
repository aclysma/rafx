use rafx::render_feature_extract_job_predule::*;

use super::*;
use rafx::assets::{AssetManagerExtractRef, AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_ref_map::ResourceRefBorrow;
use rafx::distill::loader::handle::Handle;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct DebugPipExtractJob<'extract> {
    asset_manager: AssetManagerExtractRef,
    debug_pip_material: Handle<MaterialAsset>,
    _debug_pip_resource: ResourceRefBorrow<'extract, DebugPipResource>,
    phantom_data: PhantomData<&'extract ()>,
}

impl<'extract> DebugPipExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<DebugPipFramePacket>,
        debug_pip_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>()
                    .extract_ref(),
                _debug_pip_resource: extract_context
                    .extract_resources
                    .fetch::<DebugPipResource>(),
                debug_pip_material,
                phantom_data: PhantomData,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for DebugPipExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        context
            .frame_packet()
            .per_frame_data()
            .set(DebugPipPerFrameData {
                debug_pip_material_pass: self
                    .asset_manager
                    .committed_asset(&self.debug_pip_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
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

    type FramePacketDataT = DebugPipRenderFeatureTypes;
}
