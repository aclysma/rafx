use rafx::render_feature_renderer_prelude::*;

use super::*;
use crate::features::mesh_adv::light_binning::MeshAdvLightBinRenderResource;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase, TransparentRenderPhase,
    WireframeRenderPhase,
};
use distill::loader::handle::Handle;
use rafx::assets::{ComputePipelineAsset, MaterialAsset};

pub struct MeshAdvStaticResources {
    pub depth_material: Handle<MaterialAsset>,
    pub shadow_map_atlas_depth_material: Handle<MaterialAsset>,
    pub shadow_map_atlas_clear_tiles_material: Handle<MaterialAsset>,
    pub lights_bin_compute_pipeline: Handle<ComputePipelineAsset>,
    pub lights_build_lists_compute_pipeline: Handle<ComputePipelineAsset>,
}

pub struct MeshAdvRendererPlugin {
    render_objects: MeshAdvRenderObjectSet,
    max_num_mesh_parts: Option<usize>,
}

impl MeshAdvRendererPlugin {
    pub fn new(max_num_mesh_parts: Option<usize>) -> Self {
        Self {
            max_num_mesh_parts,
            render_objects: MeshAdvRenderObjectSet::default(),
        }
    }
}

#[cfg(feature = "legion")]
impl MeshAdvRendererPlugin {
    pub fn legion_init(
        &self,
        resources: &mut legion::Resources,
    ) {
        resources.insert(self.render_objects.clone());
    }

    pub fn legion_destroy(resources: &mut legion::Resources) {
        resources.remove::<MeshAdvRenderObjectSet>();
    }
}

impl RenderFeaturePlugin for MeshAdvRendererPlugin {
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
            .register_feature::<MeshAdvRenderFeature>()
            .register_feature_flag::<MeshAdvWireframeRenderFeatureFlag>()
            .register_feature_flag::<MeshAdvUntexturedRenderFeatureFlag>()
            .register_feature_flag::<MeshAdvUnlitRenderFeatureFlag>()
            .register_feature_flag::<MeshAdvNoShadowsRenderFeatureFlag>()
    }

    fn initialize_static_resources(
        &self,
        asset_manager: &mut AssetManager,
        asset_resource: &mut AssetResource,
        _extract_resources: &ExtractResources,
        render_resources: &mut ResourceMap,
        _upload: &mut RafxTransferUpload,
    ) -> RafxResult<()> {
        let depth_material = asset_resource
            .load_asset_path::<MaterialAsset, _>("rafx-plugins/materials/depth.material");

        let shadow_map_atlas_depth_material = asset_resource.load_asset_path::<MaterialAsset, _>(
            "rafx-plugins/materials/modern_pipeline/shadow_atlas_depth.material",
        );

        let shadow_map_atlas_clear_tiles_material = asset_resource
            .load_asset_path::<MaterialAsset, _>(
                "rafx-plugins/materials/modern_pipeline/shadow_atlas_clear_tiles.material",
            );

        let lights_bin_compute_pipeline = asset_resource
            .load_asset_path::<ComputePipelineAsset, _>(
                "rafx-plugins/compute_pipelines/lights_bin.compute",
            );

        let lights_build_lists_compute_pipeline = asset_resource
            .load_asset_path::<ComputePipelineAsset, _>(
                "rafx-plugins/compute_pipelines/lights_build_lists.compute",
            );

        asset_manager.wait_for_asset_to_load(&depth_material, asset_resource, "depth")?;
        asset_manager.wait_for_asset_to_load(
            &shadow_map_atlas_depth_material,
            asset_resource,
            "shadow atlas depth",
        )?;
        asset_manager.wait_for_asset_to_load(
            &shadow_map_atlas_clear_tiles_material,
            asset_resource,
            "shadow atlas clear",
        )?;
        asset_manager.wait_for_asset_to_load(
            &lights_bin_compute_pipeline,
            asset_resource,
            "lights_bin.compute",
        )?;
        asset_manager.wait_for_asset_to_load(
            &lights_build_lists_compute_pipeline,
            asset_resource,
            "lights_build_lists.compute",
        )?;

        render_resources.insert(MeshAdvStaticResources {
            depth_material,
            shadow_map_atlas_depth_material,
            shadow_map_atlas_clear_tiles_material,
            lights_bin_compute_pipeline,
            lights_build_lists_compute_pipeline,
        });

        render_resources.insert(MeshAdvShadowMapResource::default());

        let lights_bin_render_resource = MeshAdvLightBinRenderResource::new(
            &asset_manager.resource_manager().resource_context(),
        )?;
        render_resources.insert(lights_bin_render_resource);

        render_resources.insert(ShadowMapAtlas::new(asset_manager.resources())?);

        Ok(())
    }

    fn prepare_renderer_destroy(
        &self,
        render_resources: &ResourceMap,
    ) -> RafxResult<()> {
        // Clear shadow map assignments so that all shadow map atlas elements are free
        let mut shadow_map_resource = render_resources.fetch_mut::<MeshAdvShadowMapResource>();
        shadow_map_resource.clear();

        Ok(())
    }

    fn add_render_views(
        &self,
        extract_resources: &ExtractResources,
        render_resources: &RenderResources,
        render_view_set: &RenderViewSet,
        render_views: &mut Vec<RenderView>,
    ) {
        //TODO: HACK
        let main_view_eye_position = render_views[0].eye_position();
        let mut shadow_map_resource = render_resources.fetch_mut::<MeshAdvShadowMapResource>();
        let mut shadow_map_atlas = render_resources.fetch_mut::<ShadowMapAtlas>();
        shadow_map_resource.recalculate_shadow_map_views(
            &render_view_set,
            extract_resources,
            &mut *shadow_map_atlas,
            main_view_eye_position,
        );

        shadow_map_resource.append_render_views(render_views);
    }

    fn new_frame_packet(
        &self,
        frame_packet_size: &FramePacketSize,
    ) -> Box<dyn RenderFeatureFramePacket> {
        Box::new(MeshAdvFramePacket::new(
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
            .fetch::<MeshAdvStaticResources>();

        let depth_material = static_resources.depth_material.clone();

        let shadow_map_atlas_depth_material =
            static_resources.shadow_map_atlas_depth_material.clone();

        MeshAdvExtractJob::new(
            extract_context,
            frame_packet.into_concrete(),
            depth_material,
            shadow_map_atlas_depth_material,
            self.render_objects.clone(),
        )
    }

    fn new_submit_packet(
        &self,
        frame_packet: &Box<dyn RenderFeatureFramePacket>,
    ) -> Box<dyn RenderFeatureSubmitPacket> {
        let frame_packet: &MeshAdvFramePacket = frame_packet.as_ref().as_concrete();

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
                SubmitNodeBlock::with_capacity_and_feature_flag::<
                    WireframeRenderPhase,
                    MeshAdvWireframeRenderFeatureFlag,
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
        MeshAdvPrepareJob::new(
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
        MeshAdvWriteJob::new(
            write_context,
            frame_packet.into_concrete(),
            submit_packet.into_concrete(),
        )
    }
}
