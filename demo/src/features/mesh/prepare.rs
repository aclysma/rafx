use renderer::nodes::{
    RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex,
    FramePacket, DefaultPrepareJobImpl, PerFrameNode, PerViewNode, RenderFeature, RenderViewIndex
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

pub struct MeshPrepareJobImpl {
    pub(super) descriptor_set_allocator: DescriptorSetAllocatorRef,

    pub(super) extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub(super) directional_lights: Vec<DirectionalLightComponent>,
    pub(super) point_lights: Vec<(PositionComponent, PointLightComponent)>,
    pub(super) spot_lights: Vec<(PositionComponent, SpotLightComponent)>,

    //TODO: reserve sizes
    pub(super) per_view_descriptor_set_layouts: FnvHashSet<ResourceArc<DescriptorSetLayoutResource>>,
    pub(super) prepared_submit_node_mesh_data: Vec<PreparedSubmitNodeMeshData>,
    pub(super) per_view_descriptor_sets: FnvHashMap<(RenderViewIndex, ResourceArc<DescriptorSetLayoutResource>), DescriptorSetArc>
}

impl MeshPrepareJobImpl {
    pub(super) fn new(
        descriptor_set_allocator: DescriptorSetAllocatorRef,
        extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
        directional_lights: Vec<DirectionalLightComponent>,
        point_lights: Vec<(PositionComponent, PointLightComponent)>,
        spot_lights: Vec<(PositionComponent, SpotLightComponent)>,
    ) -> Self {
        MeshPrepareJobImpl {
            descriptor_set_allocator,
            extracted_frame_node_mesh_data,
            directional_lights,
            point_lights,
            spot_lights,
            per_view_descriptor_set_layouts: Default::default(),
            prepared_submit_node_mesh_data: Default::default(),
            per_view_descriptor_sets: Default::default(),
        }
    }
}

impl DefaultPrepareJobImpl<RenderJobPrepareContext, RenderJobWriteContext> for MeshPrepareJobImpl {
    fn prepare_begin(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[&RenderView],
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {
        for mesh in &self.extracted_frame_node_mesh_data {
            if let Some(mesh) = mesh {
                for mesh_part in &*mesh.mesh_asset.inner.mesh_parts {
                    self.per_view_descriptor_set_layouts.insert(mesh_part.material_passes[0].descriptor_set_layouts[PER_VIEW_LAYOUT_INDEX].clone());
                }
            }
        }

        for &view in views {
            let view_data = self.create_per_view_data(view);

            for per_view_descriptor_set_layout in &self.per_view_descriptor_set_layouts {
                let mut descriptor_set = self.descriptor_set_allocator
                    .create_dyn_descriptor_set_uninitialized(&per_view_descriptor_set_layout)
                    .unwrap();
                descriptor_set.set_buffer_data(0, &view_data);
                descriptor_set
                    .flush(&mut self.descriptor_set_allocator)
                    .unwrap();

                self.per_view_descriptor_sets.insert((view.view_index(), per_view_descriptor_set_layout.clone()), descriptor_set.descriptor_set().clone());
            }
        }
    }

    fn prepare_frame_node(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        _frame_node: PerFrameNode,
        _frame_node_index: u32,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {
    }

    fn prepare_view_node(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    ) {
        let extracted_data = &self.extracted_frame_node_mesh_data[view_node.frame_node_index() as usize];
        if let Some(extracted_data) = extracted_data {
            let model_view = view.view_matrix() * extracted_data.world_transform;
            let model_view_proj = view.projection_matrix() * model_view;

            let per_object_param = MeshPerObjectShaderParam {
                model_view,
                model_view_proj,
            };

            for (mesh_part_index, mesh_part) in extracted_data.mesh_asset.inner.mesh_parts.iter().enumerate() {

                // Stash the layout for building descriptor sets per-view later
                //self.per_view_descriptor_set_layouts.insert(mesh_part.material_passes[0].descriptor_set_layouts[PER_VIEW_LAYOUT_INDEX].clone());








                let per_view_descriptor_set_layout = mesh_part.material_passes[0].descriptor_set_layouts[PER_VIEW_LAYOUT_INDEX].clone();
                let per_view_descriptor_set = self.per_view_descriptor_sets[&(view.view_index(), per_view_descriptor_set_layout)].clone();

                //
                // Per instance descriptor set
                //
                let per_instance_descriptor_set_layout = &mesh_part.material_passes[0].descriptor_set_layouts[PER_INSTANCE_LAYOUT_INDEX];

                // Create the per-instance descriptor set
                let mut descriptor_set = self.descriptor_set_allocator
                    .create_dyn_descriptor_set_uninitialized(per_instance_descriptor_set_layout)
                    .unwrap();
                descriptor_set.set_buffer_data(0, &per_object_param);
                descriptor_set
                    .flush(&mut self.descriptor_set_allocator)
                    .unwrap();

                let per_instance_descriptor_set = descriptor_set.descriptor_set().clone();

                //
                // Create the submit node
                //
                let submit_node_index = self.prepared_submit_node_mesh_data.len();

                self.prepared_submit_node_mesh_data.push(PreparedSubmitNodeMeshData {
                    per_view_descriptor_set,
                    per_instance_descriptor_set,
                    frame_node_index: view_node.frame_node_index(),
                    mesh_part_index,
                });
                submit_nodes.add_submit_node::<OpaqueRenderPhase>(submit_node_index as u32, 0, 0.0);
            }
        }
    }

    fn prepare_view_finalize(
        &mut self,
        _prepare_context: &RenderJobPrepareContext,
        view: &RenderView,
        _submit_nodes: &mut ViewSubmitNodes,
    ) {

    }

    fn prepare_frame_finalize(
        self,
        _prepare_context: &RenderJobPrepareContext,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<RenderJobWriteContext>> {
        Box::new(MeshCommandWriter {
            extracted_frame_node_mesh_data: self.extracted_frame_node_mesh_data,
            prepared_submit_node_mesh_data: self.prepared_submit_node_mesh_data
        })
    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}

impl MeshPrepareJobImpl {
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

