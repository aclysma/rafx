use rafx::render_feature_renderer_prelude::*;

use super::*;

use crate::phases::TransparentRenderPhase;
use hydrate_base::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::renderer::RendererLoadContext;

pub struct TileLayerStaticResources {
    pub tile_layer_material: Handle<MaterialAsset>,
}

#[derive(Default)]
pub struct TileLayerRendererPlugin {
    render_objects: TileLayerRenderObjectSet,
}

#[cfg(feature = "legion")]
impl TileLayerRendererPlugin {
    pub fn legion_init(
        &self,
        resources: &mut legion::Resources,
    ) {
        resources.insert(self.render_objects.clone());
        resources.insert(TileLayerResource::default());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<TileLayerRenderObjectSet>();
        resources.remove::<TileLayerResource>();
    }
}

impl RenderFeaturePlugin for TileLayerRendererPlugin {
    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    fn is_view_relevant(
        &self,
        view: &RenderView,
    ) -> bool {
        view.phase_is_relevant::<TransparentRenderPhase>()
    }

    fn requires_visible_render_objects(&self) -> bool {
        true
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<TileLayerRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        renderer_load_context: &RendererLoadContext,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut RenderResources,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let tile_layer_material = asset_resource.load_artifact_symbol_name::<MaterialAsset>(
            "assets://rafx-plugins/materials/tile_layer.material",
        );

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &tile_layer_material,
            asset_resource,
            "tile_layer_material",
        )?;

        render_resources.insert(TileLayerStaticResources {
            tile_layer_material,
        });

        Ok(())
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(TileLayerFramePacket::new(
            self.feature_index(),
            frame_packet_size,
        ))
    }

    fn new_extract_job<'extract>(
        &self,
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        let tile_layer_material = extract_context
            .render_resources
            .fetch::<TileLayerStaticResources>()
            .tile_layer_material
            .clone();

        TileLayerExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            tile_layer_material,
            self.render_objects.clone(),
        )
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &TileLayerFramePacket = frame_packet.as_ref().as_concrete();
        let num_submit_nodes = frame_packet.render_object_instances().len();

        let mut view_submit_packets = Vec::with_capacity(frame_packet.view_packets().len());
        for view_packet in frame_packet.view_packets() {
            let view_submit_packet = ViewSubmitPacket::from_view_packet::<TransparentRenderPhase>(
                view_packet,
                Some(num_submit_nodes),
            );
            view_submit_packets.push(view_submit_packet);
        }

        Box::new(TileLayerSubmitPacket::new(
            self.feature_index(),
            frame_packet.render_object_instances().len(),
            view_submit_packets,
        ))
    }

    fn new_prepare_job<'prepare>(
        &self,
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
        submit_packet: Box<dyn RenderFeatureSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        TileLayerPrepareJob::new(
            prepare_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
            self.render_objects.clone(),
        )
    }

    fn new_write_job<'write>(
        &self,
        write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
        submit_packet: Box<dyn RenderFeatureSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        TileLayerWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
            self.render_objects.clone(),
        )
    }
}
