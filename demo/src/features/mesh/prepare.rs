use renderer::nodes::{
    RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex,
    FramePacket, PrepareJob, PerFrameNode, PerViewNode, RenderFeature, RenderViewIndex
};
use crate::features::mesh::{MeshRenderFeature, ExtractedFrameNodeMeshData, DirectionalLight, MeshPerViewShaderParam, MeshPerObjectShaderParam, PreparedSubmitNodeMeshData};
use crate::phases::OpaqueRenderPhase;
use glam::Vec3;
use super::MeshCommandWriter;
use crate::render_contexts::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer::assets::resources::{DescriptorSetArc, ResourceArc, GraphicsPipelineResource, DescriptorSetLayoutResource, DescriptorSetAllocatorRef};
use crate::components::{DirectionalLightComponent, PointLightComponent, SpotLightComponent, PositionComponent};
use fnv::{FnvHashSet, FnvHashMap};

const PER_VIEW_LAYOUT_INDEX : usize = 0;
const PER_INSTANCE_LAYOUT_INDEX : usize = 2;

pub struct MeshPrepareJob {
    pub(super) extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub(super) directional_lights: Vec<DirectionalLightComponent>,
    pub(super) point_lights: Vec<(PositionComponent, PointLightComponent)>,
    pub(super) spot_lights: Vec<(PositionComponent, SpotLightComponent)>,
}

impl MeshPrepareJob {
    pub(super) fn new(
        extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
        directional_lights: Vec<DirectionalLightComponent>,
        point_lights: Vec<(PositionComponent, PointLightComponent)>,
        spot_lights: Vec<(PositionComponent, SpotLightComponent)>,
    ) -> Self {
        MeshPrepareJob {
            extracted_frame_node_mesh_data,
            directional_lights,
            point_lights,
            spot_lights,
        }
    }
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
        let mut descriptor_set_allocator = prepare_context.resource_context.create_descriptor_set_allocator();

        //TODO: reserve sizes
        let mut per_view_descriptor_set_layouts = FnvHashSet::<ResourceArc<DescriptorSetLayoutResource>>::default();
        let mut prepared_submit_node_mesh_data = Vec::<PreparedSubmitNodeMeshData>::default();
        let mut per_view_descriptor_sets = FnvHashMap::<(RenderViewIndex, ResourceArc<DescriptorSetLayoutResource>), DescriptorSetArc>::default();

        let mut submit_nodes = FeatureSubmitNodes::default();
        for mesh in &self.extracted_frame_node_mesh_data {
            if let Some(mesh) = mesh {
                for mesh_part in &*mesh.mesh_asset.inner.mesh_parts {
                    per_view_descriptor_set_layouts.insert(mesh_part.material_passes[0].descriptor_set_layouts[PER_VIEW_LAYOUT_INDEX].clone());
                }
            }
        }

        for &view in views {
            let view_data = self.create_per_view_data(view);

            for per_view_descriptor_set_layout in &per_view_descriptor_set_layouts {
                let mut descriptor_set = descriptor_set_allocator
                    .create_dyn_descriptor_set_uninitialized(&per_view_descriptor_set_layout)
                    .unwrap();
                descriptor_set.set_buffer_data(0, &view_data);
                descriptor_set
                    .flush(&mut descriptor_set_allocator)
                    .unwrap();

                per_view_descriptor_sets.insert((view.view_index(), per_view_descriptor_set_layout.clone()), descriptor_set.descriptor_set().clone());
            }
        }

        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());

            let view_nodes = frame_packet.view_nodes(view, self.feature_index());
            if let Some(view_nodes) = view_nodes {
                for (view_node_index, view_node) in view_nodes.iter().enumerate() {

                    let extracted_data = &self.extracted_frame_node_mesh_data[view_node.frame_node_index() as usize];
                    if let Some(extracted_data) = extracted_data {
                        let model_view = view.view_matrix() * extracted_data.world_transform;
                        let model_view_proj = view.projection_matrix() * model_view;

                        let per_object_param = MeshPerObjectShaderParam {
                            model_view,
                            model_view_proj,
                        };

                        for (mesh_part_index, mesh_part) in extracted_data.mesh_asset.inner.mesh_parts.iter().enumerate() {
                            //
                            // Find the per-view descriptor set that matches the material used for this mesh part
                            //
                            let per_view_descriptor_set_layout = mesh_part.material_passes[0].descriptor_set_layouts[PER_VIEW_LAYOUT_INDEX].clone();
                            let per_view_descriptor_set = per_view_descriptor_sets[&(view.view_index(), per_view_descriptor_set_layout)].clone();

                            //
                            // Create the per-instance descriptor set
                            // TODO: Common case is that parts in the same mesh use same material, so only create new descriptor set if the material is
                            // different between parts.
                            //
                            let per_instance_descriptor_set_layout = &mesh_part.material_passes[0].descriptor_set_layouts[PER_INSTANCE_LAYOUT_INDEX];

                            let mut descriptor_set = descriptor_set_allocator
                                .create_dyn_descriptor_set_uninitialized(per_instance_descriptor_set_layout)
                                .unwrap();
                            descriptor_set.set_buffer_data(0, &per_object_param);
                            descriptor_set
                                .flush(&mut descriptor_set_allocator)
                                .unwrap();

                            let per_instance_descriptor_set = descriptor_set.descriptor_set().clone();

                            //
                            // Create the submit node
                            //
                            let submit_node_index = prepared_submit_node_mesh_data.len();

                            prepared_submit_node_mesh_data.push(PreparedSubmitNodeMeshData {
                                per_view_descriptor_set,
                                per_instance_descriptor_set,
                                frame_node_index: view_node.frame_node_index(),
                                mesh_part_index,
                            });

                            view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(submit_node_index as u32, 0, 0.0);
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
    fn create_per_view_data(&self, view: &RenderView) -> MeshPerViewShaderParam {
        let mut per_view_data = MeshPerViewShaderParam::default();
        for light in &self.directional_lights {
            let light_count = per_view_data.directional_light_count as usize;
            if light_count > per_view_data.directional_lights.len() {
                break;
            }

            let light_from = glam::Vec3::new(0.0, 0.0, 0.0);
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
}

