use rafx::render_feature_renderer_prelude::*;

use super::*;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase, TransparentRenderPhase,
    WireframeRenderPhase,
};
use distill::loader::handle::Handle;
use rafx::assets::MaterialAsset;
use rafx::renderer::RendererLoadContext;

pub struct MeshBasicStaticResources {
    pub default_pbr_material: Handle<MaterialAsset>,
    pub depth_material: Handle<MaterialAsset>,
}

pub struct MeshBasicRendererPlugin {
    render_objects: MeshBasicRenderObjectSet,
    max_num_mesh_parts: Option<usize>,
}

impl MeshBasicRendererPlugin {
    pub fn new(max_num_mesh_parts: Option<usize>) -> Self {
        Self {
            max_num_mesh_parts,
            render_objects: MeshBasicRenderObjectSet::default(),
        }
    }
}

#[cfg(feature = "legion")]
impl MeshBasicRendererPlugin {
    pub fn legion_init(
        &self,
        resources: &mut legion::Resources,
    ) {
        resources.insert(self.render_objects.clone());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<MeshBasicRenderObjectSet>();
    }
}

impl RenderFeaturePlugin for MeshBasicRendererPlugin {
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
            || view.phase_is_relevant::<WireframeRenderPhase>()
    }

    fn requires_visible_render_objects(&self) -> bool {
        true
    }

    fn configure_render_registry(
        &self,
        render_registry: RenderRegistryBuilder,
    ) -> RenderRegistryBuilder {
        render_registry
            .register_feature::<MeshBasicRenderFeature>()
            .register_feature_flag::<MeshBasicWireframeRenderFeatureFlag>()
            .register_feature_flag::<MeshBasicUntexturedRenderFeatureFlag>()
            .register_feature_flag::<MeshBasicUnlitRenderFeatureFlag>()
            .register_feature_flag::<MeshBasicNoShadowsRenderFeatureFlag>()
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
        let default_pbr_material = asset_resource.load_asset_path::<MaterialAsset, _>(
            "rafx-plugins/materials/basic_pipeline/mesh_basic.material",
        );

        let depth_material = asset_resource.load_asset_path::<MaterialAsset, _>(
            "rafx-plugins/materials/basic_pipeline/depth.material",
        );

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &default_pbr_material,
            asset_resource,
            "default_pbr_material",
        )?;

        renderer_load_context.wait_for_asset_to_load(
            render_resources,
            asset_manager,
            &depth_material,
            asset_resource,
            "depth",
        )?;

        render_resources.insert(MeshBasicStaticResources {
            default_pbr_material,
            depth_material,
        });

        render_resources.insert(MeshBasicShadowMapResource::default());

        Ok(())
    }

    fn add_render_views(
        &self,
        extract_resources: &ExtractResources,
        render_resources: &RenderResources,
        render_view_set: &RenderViewSet,
        render_views: &mut Vec<RenderView>,
    ) {
        let mut shadow_map_resource = render_resources.fetch_mut::<MeshBasicShadowMapResource>();
        shadow_map_resource.recalculate_shadow_map_views(&render_view_set, extract_resources);

        shadow_map_resource.append_render_views(render_views);
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(MeshBasicFramePacket::new(
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
            .fetch::<MeshBasicStaticResources>();

        let default_pbr_material = static_resources.default_pbr_material.clone();
        let depth_material = static_resources.depth_material.clone();

        MeshBasicExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            default_pbr_material,
            depth_material,
            self.render_objects.clone(),
        )
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &MeshBasicFramePacket = frame_packet.as_ref().as_concrete();

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
                SubmitNodeBlock::with_capacity::<TransparentRenderPhase>(view, num_submit_nodes),
                SubmitNodeBlock::with_capacity::<DepthPrepassRenderPhase>(view, num_submit_nodes),
                SubmitNodeBlock::with_capacity::<ShadowMapRenderPhase>(view, num_submit_nodes),
                SubmitNodeBlock::with_capacity_and_feature_flag::<
                    WireframeRenderPhase,
                    MeshBasicWireframeRenderFeatureFlag,
                >(view, num_submit_nodes),
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
        MeshBasicPrepareJob::new(
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
        MeshBasicWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
