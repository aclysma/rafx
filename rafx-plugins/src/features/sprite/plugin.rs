use rafx::render_feature_renderer_prelude::*;

use super::*;
use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase};
use hydrate_base::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::renderer::RendererLoadContext;

pub struct SpriteStaticResources {
    pub sprite_material: Handle<MaterialAsset>,
}

#[derive(Default)]
pub struct SpriteRendererPlugin {
    render_objects: SpriteRenderObjectSet,
}

#[cfg(feature = "legion")]
impl SpriteRendererPlugin {
    pub fn legion_init(
        &self,
        resources: &mut legion::Resources,
    ) {
        resources.insert(self.render_objects.clone());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<SpriteRenderObjectSet>();
    }
}

impl RenderFeaturePlugin for SpriteRendererPlugin {
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
        view.phase_is_relevant::<OpaqueRenderPhase>()
            || view.phase_is_relevant::<TransparentRenderPhase>()
    }

    fn requires_visible_render_objects(&self) -> bool {
        true
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<SpriteRenderFeature>()
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
        let sprite_material = asset_resource.load_artifact_symbol_name::<MaterialAsset>(
            "assets://rafx-plugins/materials/sprite.material",
        );

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &sprite_material,
            asset_resource,
            "sprite_material",
        )?;

        render_resources.insert(SpriteStaticResources { sprite_material });

        Ok(())
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(SpriteFramePacket::new(
            self.feature_index(),
            frame_packet_size,
        ))
    }

    fn new_extract_job<'extract>(
        &self,
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        let sprite_material = extract_context
            .render_resources
            .fetch::<SpriteStaticResources>()
            .sprite_material
            .clone();

        SpriteExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            sprite_material,
            self.render_objects.clone(),
        )
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &SpriteFramePacket = frame_packet.as_ref().as_concrete();
        let num_submit_nodes = frame_packet.render_object_instances().len();

        let mut view_submit_packets = Vec::with_capacity(frame_packet.view_packets().len());
        for view_packet in frame_packet.view_packets() {
            let view = view_packet.view();
            let submit_node_blocks = vec![
                SubmitNodeBlock::with_capacity::<OpaqueRenderPhase>(view, num_submit_nodes),
                SubmitNodeBlock::with_capacity::<TransparentRenderPhase>(view, num_submit_nodes),
            ];

            view_submit_packets.push(ViewSubmitPacket::new(
                submit_node_blocks,
                &ViewPacketSize::size_of(view_packet),
                view_packet.view_frame_index(),
            ));
        }

        Box::new(SpriteSubmitPacket::new(
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
        SpritePrepareJob::new(
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
        SpriteWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
