use rafx::render_feature_renderer_prelude::*;

use super::{ExampleExtractJob, ExampleRenderFeature};
use crate::features::internal::{ExampleFramePacket, ExampleSubmitPacket};
use crate::features::jobs::{ExamplePrepareJob, ExampleWriteJob};
use crate::phases::OpaqueRenderPhase;
use rafx::assets::MaterialAsset;
use rafx::distill::loader::handle::Handle;
use rafx_renderer::RendererLoadContext;

pub struct ExampleStaticResources {
    pub triangle_material_handle: Handle<MaterialAsset>,
}

#[derive(Default)]
pub struct ExampleRenderFeaturePlugin;

impl ExampleRenderFeaturePlugin {
    pub fn legion_init(
        &self,
        _resources: &mut legion::Resources,
    ) {
    }

    pub fn legion_destroy(_resources: &mut legion::Resources) {}
}

impl RenderFeaturePlugin for ExampleRenderFeaturePlugin {
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
        render_registry.register_feature::<ExampleRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        _renderer_load_context: &RendererLoadContext,
        _asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut RenderResources,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        //
        // Load the triangle material. Materials can contain multiple passes (but this one only has
        // one.) A material pass specifies shaders and fixed function state. Generally a material
        // is 1:1 with a GPU pipeline object with the material specifying *most* of the necessary
        // parameters to create the pipeline. (Some things like the size of the window are not
        // known until runtime.)
        //
        // When a material asset is loaded, rafx automatically creates shader modules, shaders,
        // descriptor set layouts, root signatures, and the material passes in it for you. They are
        // registered in the resource manager. We can use the handle to get the MaterialAsset which
        // has a reference to those resources. The resources will remain loaded until the handle
        // is dropped and there are no more references to those resources.
        //
        let triangle_material_handle =
            asset_resource.load_asset_path::<MaterialAsset, _>("triangle.material");

        render_resources.insert(ExampleStaticResources {
            triangle_material_handle,
        });

        Ok(())
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(ExampleFramePacket::new(
            self.feature_index(),
            frame_packet_size,
        ))
    }

    fn new_extract_job<'extract>(
        &self,
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        let triangle_material = extract_context
            .render_resources
            .fetch::<ExampleStaticResources>()
            .triangle_material_handle
            .clone();

        ExampleExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            triangle_material,
        )
    }

    #[profiling::function]
    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &ExampleFramePacket = frame_packet.as_ref().as_concrete();

        let mut view_submit_packets = Vec::with_capacity(frame_packet.view_packets().len());
        for view_packet in frame_packet.view_packets() {
            let view_submit_packet =
                ViewSubmitPacket::from_view_packet::<OpaqueRenderPhase>(view_packet, Some(1));
            view_submit_packets.push(view_submit_packet);
        }

        Box::new(ExampleSubmitPacket::new(
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
        ExamplePrepareJob::new(
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
        ExampleWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
