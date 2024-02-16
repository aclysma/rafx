use rafx::render_feature_renderer_prelude::*;

use super::*;
use crate::phases::UiRenderPhase;
use hydrate_base::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::framework::{ImageViewResource, ResourceArc};

pub struct ImGuiStaticResources {
    pub imgui_material: Handle<MaterialAsset>,
    pub imgui_font_atlas_image_view: ResourceArc<ImageViewResource>,
}

#[derive(Default)]
pub struct ImGuiRendererPlugin;

impl ImGuiRendererPlugin {
    pub fn legion_init(
        &self,
        resources: &mut legion::Resources,
        window: &sdl2::video::Window,
    ) {
        let imgui_manager = init_sdl2_imgui_manager(window);
        resources.insert(imgui_manager);
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<Sdl2ImguiManager>();
    }
}

impl RenderFeaturePlugin for ImGuiRendererPlugin {
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
        render_registry.register_feature::<ImGuiRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        extract_resources: &ExtractResources,
        render_resources: &mut RenderResources,
        upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let imgui_material = asset_resource.load_artifact_symbol_name::<MaterialAsset, _>(
            "rafx-plugins://materials/imgui.material",
        );

        asset_manager.wait_for_asset_to_load(&imgui_material, asset_resource, "imgui material")?;

        let imgui_font_atlas_data = extract_resources
            .fetch::<Sdl2ImguiManager>()
            .build_font_atlas();

        let dyn_resource_allocator = asset_manager.create_dyn_resource_allocator_set();
        let imgui_font_atlas_image_view = create_font_atlas_image_view(
            imgui_font_atlas_data,
            asset_manager.device_context(),
            upload,
            &dyn_resource_allocator,
        )?;

        render_resources.insert(ImGuiStaticResources {
            imgui_material,
            imgui_font_atlas_image_view,
        });

        Ok(())
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(ImGuiFramePacket::new(
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
            .fetch::<ImGuiStaticResources>();

        let imgui_material = static_resources.imgui_material.clone();

        ImGuiExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            imgui_material,
        )
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &ImGuiFramePacket = frame_packet.as_ref().as_concrete();

        let mut view_submit_packets = Vec::with_capacity(frame_packet.view_packets().len());
        for view_packet in frame_packet.view_packets() {
            let view_submit_packet =
                ViewSubmitPacket::from_view_packet::<UiRenderPhase>(view_packet, Some(1));
            view_submit_packets.push(view_submit_packet);
        }

        Box::new(ImGuiSubmitPacket::new(
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
        let static_resources = prepare_context
            .render_resources
            .fetch::<ImGuiStaticResources>();

        let font_atlas = static_resources.imgui_font_atlas_image_view.clone();

        ImGuiPrepareJob::new(
            prepare_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
            font_atlas,
        )
    }

    fn new_write_job<'write>(
        &self,
        write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
        submit_packet: Box<dyn RenderFeatureSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        ImGuiWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
