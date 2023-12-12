use rafx::render_feature_renderer_prelude::*;

use super::*;
use crate::phases::OpaqueRenderPhase;
use hydrate_base::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::renderer::RendererLoadContext;

pub struct SkyboxStaticResources {
    pub skybox_material: Handle<MaterialAsset>,
}

#[derive(Default)]
pub struct SkyboxRendererPlugin;

#[cfg(feature = "legion")]
impl SkyboxRendererPlugin {
    pub fn legion_init(
        &self,
        resources: &mut legion::Resources,
    ) {
        resources.insert(SkyboxResource::default());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<SkyboxResource>();
    }
}

impl RenderFeaturePlugin for SkyboxRendererPlugin {
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
    }

    fn requires_visible_render_objects(&self) -> bool {
        false
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<SkyboxRenderFeature>()
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
        let skybox_material = asset_resource.load_asset_symbol_name::<MaterialAsset>(
            "assets://rafx-plugins/materials/skybox.material",
        );

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &skybox_material,
            asset_resource,
            "skybox material",
        )?;

        render_resources.insert(SkyboxStaticResources { skybox_material });

        Ok(())
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(SkyboxFramePacket::new(
            self.feature_index(),
            frame_packet_size,
        ))
    }

    fn new_extract_job<'extract>(
        &self,
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        let static_resources = extract_context
            .render_resources
            .fetch::<SkyboxStaticResources>();

        SkyboxExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            static_resources.skybox_material.clone(),
        )
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &SkyboxFramePacket = frame_packet.as_ref().as_concrete();

        let mut view_submit_packets = Vec::with_capacity(frame_packet.view_packets().len());
        for view_packet in frame_packet.view_packets() {
            let view_submit_packet =
                ViewSubmitPacket::from_view_packet::<OpaqueRenderPhase>(view_packet, Some(1));
            view_submit_packets.push(view_submit_packet);
        }

        Box::new(SkyboxSubmitPacket::new(
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
        SkyboxPrepareJob::new(
            prepare_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }

    fn new_write_job<'write>(
        &self,
        write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
        submit_packet: Box<dyn RenderFeatureSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        SkyboxWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
