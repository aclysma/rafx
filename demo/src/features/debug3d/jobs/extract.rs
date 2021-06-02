use rafx::render_feature_extract_job_predule::*;

use super::*;
use rafx::assets::{AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_map::ReadBorrow;
use rafx::base::resource_ref_map::ResourceRefBorrowMut;
use rafx::distill::loader::handle::Handle;

pub struct Debug3DExtractJob<'extract> {
    debug3d_resource: TrustCell<ResourceRefBorrowMut<'extract, Debug3DResource>>,
    asset_manager: ReadBorrow<'extract, AssetManagerRenderResource>,
    debug3d_material: Handle<MaterialAsset>,
}

impl<'extract> Debug3DExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<Debug3DFramePacket>,
        debug3d_material: Handle<MaterialAsset>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                debug3d_resource: TrustCell::new(
                    extract_context
                        .extract_resources
                        .fetch_mut::<Debug3DResource>(),
                ),
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>(),
                debug3d_material,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for Debug3DExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        let debug3d_resource_mut = &mut self.debug3d_resource.borrow_mut();
        context
            .frame_packet()
            .per_frame_data()
            .set(Debug3DPerFrameData {
                debug3d_material_pass: self
                    .asset_manager
                    .get()
                    .unwrap()
                    .committed_asset(&self.debug3d_material)
                    .unwrap()
                    .get_single_material_pass()
                    .ok(),
                line_lists: debug3d_resource_mut.take_line_lists(),
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

    type FramePacketDataT = Debug3DRenderFeatureTypes;
}
