use rafx::render_feature_renderer_prelude::*;

use super::*;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase, TransparentRenderPhase,
};
use distill::loader::handle::Handle;
use rafx::assets::MaterialAsset;

pub struct MeshStaticResources {
    pub depth_material: Handle<MaterialAsset>,
}

pub struct MeshRendererPlugin {
    render_objects: MeshRenderObjectSet,
    max_num_mesh_parts: Option<usize>,
}

impl MeshRendererPlugin {
    pub fn new(max_num_mesh_parts: Option<usize>) -> Self {
        Self {
            max_num_mesh_parts,
            render_objects: MeshRenderObjectSet::default(),
        }
    }

    pub fn legion_init(
        &self,
        resources: &mut legion::Resources,
    ) {
        resources.insert(self.render_objects.clone());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<MeshRenderObjectSet>();
    }
}

impl RenderFeaturePlugin for MeshRendererPlugin {
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
        view.phase_is_relevant::<DepthPrepassRenderPhase>()
            || view.phase_is_relevant::<ShadowMapRenderPhase>()
            || view.phase_is_relevant::<OpaqueRenderPhase>()
            || view.phase_is_relevant::<TransparentRenderPhase>()
    }

    fn requires_visible_render_objects(&self) -> bool {
        true
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry.register_feature::<MeshRenderFeature>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let depth_material =
            asset_resource.load_asset_path::<MaterialAsset, _>("materials/depth.material");

        asset_manager.wait_for_asset_to_load(&depth_material, asset_resource, "depth")?;

        render_resources.insert(MeshStaticResources { depth_material });

        render_resources.insert(ShadowMapResource::default());

        Ok(())
    }

    fn add_render_views(
        &self,
        extract_resources: &ExtractResources,
        render_resources: &RenderResources,
        render_view_set: &RenderViewSet,
        render_views: &mut Vec<RenderView>,
    ) {
        let mut shadow_map_resource = render_resources.fetch_mut::<ShadowMapResource>();
        shadow_map_resource.recalculate_shadow_map_views(&render_view_set, extract_resources);

        shadow_map_resource.append_render_views(render_views);
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(MeshFramePacket::new(
            self.feature_index(),
            frame_packet_size,
        ))
    }

    fn new_extract_job<'extract>(
        &self,
        extract_context: &RenderJobExtractContext<'extract>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
    ) -> Arc<dyn RenderFeatureExtractJob<'extract> + 'extract> {
        let depth_material = extract_context
            .render_resources
            .fetch::<MeshStaticResources>()
            .depth_material
            .clone();

        MeshExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            depth_material,
            self.render_objects.clone(),
        )
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &MeshFramePacket = frame_packet.as_ref().as_concrete();

        let mut view_submit_packets = Vec::with_capacity(frame_packet.view_packets().len());
        for view_packet in frame_packet.view_packets() {
            let num_submit_nodes = if let Some(max_num_mesh_parts) = self.max_num_mesh_parts {
                view_packet.num_render_object_instances() * max_num_mesh_parts
            } else {
                // TODO(dvd): Count exact number of submit nodes required.
                todo!()
            };

            let view = view_packet.view();
            let submit_node_blocks = vec![
                SubmitNodeBlock::with_capacity::<OpaqueRenderPhase>(view, num_submit_nodes),
                SubmitNodeBlock::with_capacity::<DepthPrepassRenderPhase>(view, num_submit_nodes),
                SubmitNodeBlock::with_capacity::<ShadowMapRenderPhase>(view, num_submit_nodes),
            ];

            view_submit_packets.push(ViewSubmitPacket::new(
                submit_node_blocks,
                &ViewPacketSize::size_of(view_packet),
            ));
        }

        Box::new(MeshSubmitPacket::new(
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
        MeshPrepareJob::new(
            prepare_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
            self.render_objects.clone(),
            self.max_num_mesh_parts,
        )
    }

    fn new_write_job<'write>(
        &self,
        write_context: &RenderJobWriteContext<'write>,
        frame_packet: Box<dyn RenderFeatureFramePacket>,
        submit_packet: Box<dyn RenderFeatureSubmitPacket>,
    ) -> Arc<dyn RenderFeatureWriteJob<'write> + 'write> {
        MeshWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
