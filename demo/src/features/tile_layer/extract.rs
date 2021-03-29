use crate::features::tile_layer::plugin::TileLayerStaticResources;
use crate::features::tile_layer::prepare::TileLayerPrepareJob;
use crate::features::tile_layer::{TileLayerRenderFeature, TileLayerRenderNodeSet, TileLayerRenderNode};
use rafx::assets::AssetManagerRenderResource;
use rafx::base::slab::RawSlabKey;
use rafx::nodes::{
    ExtractJob, FramePacket, PrepareJob, RenderFeature, RenderFeatureIndex,
    RenderJobExtractContext, RenderView,
};

pub struct TileLayerExtractJob {}

impl TileLayerExtractJob {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExtractJob for TileLayerExtractJob {
    fn extract(
        self: Box<Self>,
        extract_context: &RenderJobExtractContext,
        frame_packet: &FramePacket,
        _views: &[RenderView],
    ) -> Box<dyn PrepareJob> {
        profiling::scope!("TileLayer Extract");

        let asset_manager = extract_context
            .render_resources
            .fetch::<AssetManagerRenderResource>();

        let static_resources = extract_context
            .render_resources
            .fetch::<TileLayerStaticResources>();

        let tile_layer_material = asset_manager
            .committed_asset(&static_resources.tile_layer_material)
            .unwrap()
            .get_single_material_pass()
            .unwrap();

        let mut tile_layer_render_nodes = extract_context
            .extract_resources
            .fetch_mut::<TileLayerRenderNodeSet>();
        tile_layer_render_nodes.update();

        let mut visible_render_nodes = Vec::with_capacity(frame_packet.frame_node_count(TileLayerRenderFeature::feature_index()) as usize);
        for frame_node in frame_packet.frame_nodes(TileLayerRenderFeature::feature_index()) {
            let render_node_handle = RawSlabKey::<TileLayerRenderNode>::new(frame_node.render_node_index());
            let render_node = tile_layer_render_nodes.tile_layers.get_raw(render_node_handle).unwrap();
            visible_render_nodes.push(render_node.clone());
        }

        let prepare_impl = TileLayerPrepareJob::new(
            visible_render_nodes,
            tile_layer_material
        );

        Box::new(prepare_impl)
    }

    fn feature_debug_name(&self) -> &'static str {
        TileLayerRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        TileLayerRenderFeature::feature_index()
    }
}
