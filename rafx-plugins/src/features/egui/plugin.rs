use rafx::render_feature_renderer_prelude::*;

use super::*;
use crate::phases::UiRenderPhase;
use hydrate_base::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::renderer::RendererLoadContext;

pub struct EguiStaticResources {
    pub egui_material: Handle<MaterialAsset>,
}

#[derive(Default)]
pub struct EguiRendererPlugin;

#[cfg(feature = "legion")]
impl EguiRendererPlugin {
    #[cfg(feature = "egui-winit")]
    pub fn legion_init_winit(
        &self,
        resources: &mut legion::Resources,
    ) {
        let winit_egui_manager = WinitEguiManager::new();
        resources.insert(winit_egui_manager.egui_manager().context_resource());
        resources.insert(winit_egui_manager);
    }

    #[cfg(feature = "egui-sdl2")]
    pub fn legion_init_sdl2(
        &self,
        resources: &mut legion::Resources,
        sdl2_video_subsystem: &sdl2::VideoSubsystem,
        sdl2_mouse: sdl2::mouse::MouseUtil,
    ) {
        let sdl2_egui_manager = Sdl2EguiManager::new(sdl2_video_subsystem, sdl2_mouse);
        resources.insert(sdl2_egui_manager.egui_manager().context_resource());
        resources.insert(sdl2_egui_manager);
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        #[cfg(feature = "egui-winit")]
        resources.remove::<WinitEguiManager>();

        #[cfg(feature = "egui-sdl2")]
        resources.remove::<Sdl2EguiManager>();

        resources.remove::<EguiContextResource>();
    }
}

impl RenderFeaturePlugin for EguiRendererPlugin {
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
        view.phase_is_relevant::<UiRenderPhase>()
    }

    fn requires_visible_render_objects(&self) -> bool {
        false
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<EguiRenderFeature>()
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
        let egui_material = asset_resource
            .load_asset_path::<MaterialAsset, _>("rafx-plugins/materials/egui.material");

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &egui_material,
            asset_resource,
            "egui material",
        )?;

        render_resources.insert(EguiStaticResources { egui_material });

        // Will manage the texture atlas/generate image updates to be written later
        render_resources.insert(EguiFontAtlasCache::default());

        Ok(())
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(EguiFramePacket::new(
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
            .fetch::<EguiStaticResources>();

        let egui_material = static_resources.egui_material.clone();

        EguiExtractJob::new(extract_context, frame_packet.into_concrete(), egui_material)
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &EguiFramePacket = frame_packet.as_ref().as_concrete();

        let mut view_submit_packets = Vec::with_capacity(frame_packet.view_packets().len());
        for view_packet in frame_packet.view_packets() {
            let view_submit_packet =
                ViewSubmitPacket::from_view_packet::<UiRenderPhase>(view_packet, Some(1));
            view_submit_packets.push(view_submit_packet);
        }

        Box::new(EguiSubmitPacket::new(
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
        EguiPrepareJob::new(
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
        EguiWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
