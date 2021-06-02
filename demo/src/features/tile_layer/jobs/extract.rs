use rafx::render_feature_extract_job_predule::*;

use super::*;
use rafx::assets::{AssetManagerRenderResource, MaterialAsset};
use rafx::base::resource_map::ReadBorrow;
use rafx::distill::loader::handle::Handle;

pub struct TileLayerExtractJob<'extract> {
    asset_manager: ReadBorrow<'extract, AssetManagerRenderResource>,
    tile_layer_material: Handle<MaterialAsset>,
    render_objects: TileLayerRenderObjectSet,
}

impl<'extract> TileLayerExtractJob<'extract> {
    pub fn new(
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<TileLayerFramePacket>,
        tile_layer_material: Handle<MaterialAsset>,
        render_objects: TileLayerRenderObjectSet,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        Arc::new(ExtractJob::new(
            Self {
                asset_manager: extract_context
                    .render_resources
                    .fetch::<AssetManagerRenderResource>(),
                tile_layer_material,
                render_objects,
            },
            frame_packet,
        ))
    }
}

impl<'extract> ExtractJobEntryPoints<'extract> for TileLayerExtractJob<'extract> {
    fn begin_per_frame_extract(
        &self,
        context: &ExtractPerFrameContext<'extract, '_, Self>,
    ) {
        context
            .frame_packet()
            .per_frame_data()
            .set(TileLayerPerFrameData {
                tile_layer_material_pass: self
                    .asset_manager
                    .get()
                    .unwrap()
                    .committed_asset(&self.tile_layer_material)
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

    type FramePacketDataT = TileLayerRenderFeatureTypes;
}
