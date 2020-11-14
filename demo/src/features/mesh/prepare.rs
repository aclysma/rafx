use super::MeshCommandWriter;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::features::mesh::{
    ExtractedFrameNodeMeshData, MeshPerFrameVertexShaderParam, MeshPerObjectShaderParam,
    MeshPerViewFragmentShaderParam, MeshRenderFeature, PreparedSubmitNodeMeshData,
};
use crate::phases::{OpaqueRenderPhase, ShadowMapRenderPhase};
use crate::render_contexts::{RenderJobPrepareContext, RenderJobWriteContext};
use fnv::{FnvHashMap, FnvHashSet};
use renderer::assets::assets::MaterialPass;
use renderer::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PerViewNode, PrepareJob, RenderFeature,
    RenderFeatureIndex, RenderView, RenderViewIndex, ViewSubmitNodes,
};
use renderer::resources::{
    DescriptorSetAllocatorRef, DescriptorSetArc, DescriptorSetLayoutResource, ImageViewResource,
    ResourceArc,
};

pub struct MeshPrepareJob {
    pub(super) extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub(super) directional_lights: Vec<DirectionalLightComponent>,
    pub(super) shadow_map_image: ResourceArc<ImageViewResource>,
    pub(super) shadow_map_view: RenderView,
    pub(super) point_lights: Vec<(PositionComponent, PointLightComponent)>,
    pub(super) spot_lights: Vec<(PositionComponent, SpotLightComponent)>,
}

impl PrepareJob<RenderJobPrepareContext, RenderJobWriteContext> for MeshPrepareJob {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (
        Box<dyn FeatureCommandWriter<RenderJobWriteContext>>,
        FeatureSubmitNodes,
    ) {
        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        //TODO: reserve sizes
        let mut opaque_per_view_descriptor_set_layouts =
            FnvHashSet::<ResourceArc<DescriptorSetLayoutResource>>::default();
        let mut shadow_map_per_view_descriptor_set_layouts =
            FnvHashSet::<ResourceArc<DescriptorSetLayoutResource>>::default();
        let mut prepared_submit_node_mesh_data = Vec::<PreparedSubmitNodeMeshData>::default();
        let mut per_view_descriptor_sets = FnvHashMap::<
            (RenderViewIndex, ResourceArc<DescriptorSetLayoutResource>),
            DescriptorSetArc,
        >::default();

        //
        // Iterate every mesh part to find all the per-view descriptor sets layouts
        //
        // Realistically all the materials (for now) have identical layouts so iterating though them
        // like this just avoids hardcoding to find the first mesh's first opaque pass
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for mesh in &self.extracted_frame_node_mesh_data {
            if let Some(mesh) = mesh {
                for mesh_part in &*mesh.mesh_asset.inner.mesh_parts {
                    if let Some(mesh_part) = mesh_part {
                        opaque_per_view_descriptor_set_layouts.insert(
                            mesh_part.opaque_pass.descriptor_set_layouts
                                [super::PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                                .clone(),
                        );

                        if let Some(shadow_map_pass) = mesh_part.shadow_map_pass.as_ref() {
                            shadow_map_per_view_descriptor_set_layouts.insert(
                                shadow_map_pass.descriptor_set_layouts
                                    [super::PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                                    .clone(),
                            );
                        }
                    }
                }
            }
        }

        // Shared per-frame data
        let per_frame_vertex_data = MeshPerFrameVertexShaderParam {
            shadow_map_view_proj: self.shadow_map_view.view_proj(),
            shadow_map_light_dir: self.shadow_map_view.view_dir().extend(1.0),
        };

        //
        // Create per-view descriptors for all per-view descriptor layouts that are in our materials
        //
        for &view in views {
            let per_view_frag_data = self.create_per_view_frag_data(view);

            if view.phase_is_relevant::<OpaqueRenderPhase>() {
                for per_view_descriptor_set_layout in &opaque_per_view_descriptor_set_layouts {
                    let mut descriptor_set = descriptor_set_allocator
                        .create_dyn_descriptor_set_uninitialized(&per_view_descriptor_set_layout)
                        .unwrap();
                    descriptor_set.set_buffer_data(0, &per_view_frag_data);
                    // 1: immutable sampler
                    // 2: immutable sampler
                    descriptor_set.set_image(3, self.shadow_map_image.clone());
                    descriptor_set.set_buffer_data(4, &per_frame_vertex_data);
                    descriptor_set.flush(&mut descriptor_set_allocator).unwrap();

                    let old = per_view_descriptor_sets.insert(
                        (view.view_index(), per_view_descriptor_set_layout.clone()),
                        descriptor_set.descriptor_set().clone(),
                    );
                    assert!(old.is_none());
                }
            }

            if view.phase_is_relevant::<ShadowMapRenderPhase>() {
                for per_view_descriptor_set_layout in &shadow_map_per_view_descriptor_set_layouts {
                    let mut descriptor_set = descriptor_set_allocator
                        .create_dyn_descriptor_set_uninitialized(&per_view_descriptor_set_layout)
                        .unwrap();
                    descriptor_set.set_buffer_data(0, &per_view_frag_data);
                    descriptor_set.flush(&mut descriptor_set_allocator).unwrap();

                    let old = per_view_descriptor_sets.insert(
                        (view.view_index(), per_view_descriptor_set_layout.clone()),
                        descriptor_set.descriptor_set().clone(),
                    );
                    assert!(old.is_none());
                }
            }
        }

        //
        // Produce render nodes for every mesh
        //
        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());

            let view_nodes = frame_packet.view_nodes(view, self.feature_index());
            if let Some(view_nodes) = view_nodes {
                for view_node in view_nodes {
                    let extracted_data =
                        &self.extracted_frame_node_mesh_data[view_node.frame_node_index() as usize];
                    if let Some(extracted_data) = extracted_data {
                        let model_view = view.view_matrix() * extracted_data.world_transform;
                        let model_view_proj = view.projection_matrix() * model_view;

                        let per_object_param = MeshPerObjectShaderParam {
                            model: extracted_data.world_transform,
                            model_view,
                            model_view_proj,
                        };

                        for (mesh_part_index, mesh_part) in extracted_data
                            .mesh_asset
                            .inner
                            .mesh_parts
                            .iter()
                            .enumerate()
                        {
                            if let Some(mesh_part) = mesh_part {
                                //
                                // Write opaque render node, if it's relevant
                                //
                                if view.phase_is_relevant::<OpaqueRenderPhase>() {
                                    let submit_node_index = MeshPrepareJob::add_render_node(
                                        &mut descriptor_set_allocator,
                                        &mut prepared_submit_node_mesh_data,
                                        &per_view_descriptor_sets,
                                        &view,
                                        view_node,
                                        &per_object_param,
                                        mesh_part_index,
                                        &mesh_part.opaque_pass,
                                        &mesh_part.opaque_material_descriptor_set,
                                    );

                                    view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(
                                        submit_node_index as u32,
                                        0,
                                        0.0,
                                    );
                                }

                                //
                                // Write shadow map render node, if it's relevant
                                //
                                if let Some(shadow_map_pass) = &mesh_part.shadow_map_pass {
                                    if let Some(shadow_map_material_descriptor_set) =
                                        &mesh_part.shadow_map_material_descriptor_set
                                    {
                                        if view.phase_is_relevant::<ShadowMapRenderPhase>() {
                                            let submit_node_index = MeshPrepareJob::add_render_node(
                                                &mut descriptor_set_allocator,
                                                &mut prepared_submit_node_mesh_data,
                                                &per_view_descriptor_sets,
                                                &view,
                                                view_node,
                                                &per_object_param,
                                                mesh_part_index,
                                                shadow_map_pass,
                                                shadow_map_material_descriptor_set,
                                            );

                                            view_submit_nodes
                                                .add_submit_node::<ShadowMapRenderPhase>(
                                                    submit_node_index as u32,
                                                    0,
                                                    0.0,
                                                );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        let writer = Box::new(MeshCommandWriter {
            extracted_frame_node_mesh_data: self.extracted_frame_node_mesh_data,
            prepared_submit_node_mesh_data,
        });

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}

impl MeshPrepareJob {
    fn create_per_view_frag_data(
        &self,
        view: &RenderView,
    ) -> MeshPerViewFragmentShaderParam {
        let mut per_view_data = MeshPerViewFragmentShaderParam::default();

        per_view_data.ambient_light = glam::Vec4::new(0.03, 0.03, 0.03, 1.0);

        for light in &self.directional_lights {
            let light_count = per_view_data.directional_light_count as usize;
            if light_count > per_view_data.directional_lights.len() {
                break;
            }

            let light_from = glam::Vec3::zero();
            let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
            let light_to = light.direction;
            let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

            let light_direction = (light_to - light_from).normalize();
            let light_direction_vs = (light_to_vs - light_from_vs).normalize();

            let out = &mut per_view_data.directional_lights[light_count];
            out.direction_ws = light_direction;
            out.direction_vs = light_direction_vs;
            out.color = light.color;
            out.intensity = light.intensity;

            per_view_data.directional_light_count += 1;
        }

        for (position, light) in &self.point_lights {
            let light_count = per_view_data.point_light_count as usize;
            if light_count > per_view_data.point_lights.len() {
                break;
            }

            let out = &mut per_view_data.point_lights[light_count];
            out.position_ws = position.position;
            out.position_vs = (view.view_matrix() * position.position.extend(1.0)).truncate();
            out.color = light.color;
            out.range = light.range;
            out.intensity = light.intensity;

            per_view_data.point_light_count += 1;
        }

        for (position, light) in &self.spot_lights {
            let light_count = per_view_data.spot_light_count as usize;
            if light_count > per_view_data.spot_lights.len() {
                break;
            }

            let light_from = position.position;
            let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
            let light_to = position.position + light.direction;
            let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

            let light_direction = (light_to - light_from).normalize();
            let light_direction_vs = (light_to_vs - light_from_vs).normalize();

            let out = &mut per_view_data.spot_lights[light_count];
            out.position_ws = light_from;
            out.position_vs = light_from_vs;
            out.direction_ws = light_direction;
            out.direction_vs = light_direction_vs;
            out.spotlight_half_angle = light.spotlight_half_angle;
            out.color = light.color;
            out.range = light.range;
            out.intensity = light.intensity;

            per_view_data.spot_light_count += 1;
        }

        per_view_data
    }

    fn add_render_node(
        mut descriptor_set_allocator: &mut DescriptorSetAllocatorRef,
        prepared_submit_node_mesh_data: &mut Vec<PreparedSubmitNodeMeshData>,
        per_view_descriptor_sets: &FnvHashMap<
            (u32, ResourceArc<DescriptorSetLayoutResource>),
            DescriptorSetArc,
        >,
        view: &RenderView,
        view_node: &PerViewNode,
        per_object_param: &MeshPerObjectShaderParam,
        mesh_part_index: usize,
        material_pass: &MaterialPass,
        per_material_descriptor_set: &DescriptorSetArc,
    ) -> usize {
        let per_view_descriptor_set_layout = material_pass.descriptor_set_layouts
            [super::PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
            .clone();

        let per_view_descriptor_set =
            per_view_descriptor_sets[&(view.view_index(), per_view_descriptor_set_layout)].clone();

        //
        // Create the per-instance descriptor set
        // TODO: Common case is that parts in the same mesh use same material, so only create new descriptor set if the material is
        // different between parts.
        //
        let per_instance_descriptor_set_layout = &material_pass.descriptor_set_layouts
            [super::PER_INSTANCE_DESCRIPTOR_SET_INDEX as usize];
        let mut descriptor_set = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(per_instance_descriptor_set_layout)
            .unwrap();
        descriptor_set.set_buffer_data(0, per_object_param);
        descriptor_set.flush(&mut descriptor_set_allocator).unwrap();
        let per_instance_descriptor_set = descriptor_set.descriptor_set().clone();

        //
        // Create the submit node
        //
        let submit_node_index = prepared_submit_node_mesh_data.len();
        prepared_submit_node_mesh_data.push(PreparedSubmitNodeMeshData {
            material_pass: material_pass.clone(),
            per_view_descriptor_set,
            per_material_descriptor_set: per_material_descriptor_set.clone(),
            per_instance_descriptor_set,
            frame_node_index: view_node.frame_node_index(),
            mesh_part_index,
        });
        submit_node_index
    }
}
