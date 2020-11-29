use super::MeshCommandWriter;
use crate::features::mesh::{
    ExtractedFrameNodeMeshData, MeshPerObjectFragmentShaderParam, MeshPerViewFragmentShaderParam,
    MeshRenderFeature, PreparedDirectionalLight, PreparedPointLight, PreparedSpotLight,
    PreparedSubmitNodeMeshData, ShadowMapData,
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
    DescriptorSetAllocatorRef, DescriptorSetArc, DescriptorSetLayoutResource, ResourceArc,
};

pub struct MeshPrepareJob {
    pub(super) extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub(super) directional_lights: Vec<PreparedDirectionalLight>,
    pub(super) point_lights: Vec<PreparedPointLight>,
    pub(super) spot_lights: Vec<PreparedSpotLight>,
    pub(super) shadow_map_data: ShadowMapData,
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
        profiling::scope!("Mesh Prepare");

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        //TODO: reserve sizes
        let mut opaque_per_view_descriptor_set_layouts =
            FnvHashSet::<ResourceArc<DescriptorSetLayoutResource>>::default();
        // let mut shadow_map_per_view_descriptor_set_layouts =
        //     FnvHashSet::<ResourceArc<DescriptorSetLayoutResource>>::default();
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
                    }
                }
            }
        }

        //
        // Create uniform data for each shadow map and a properly-sized static array of the images
        //
        //TODO: Pull this const from the shader
        const MAX_SHADOW_MAPS: usize = 48;
        let mut shadow_maps = [shaders::mesh_frag::ShadowMapDataStd140::default(); MAX_SHADOW_MAPS];
        let mut shadow_map_images = [None; MAX_SHADOW_MAPS];
        for (index, shadow_map_render_view) in &mut self
            .shadow_map_data
            .shadow_map_render_views
            .iter()
            .enumerate()
        {
            if index > shadow_map_images.len() {
                log::warn!("More shadow maps than the mesh shader can support");
                break;
            }

            shadow_maps[index] = shaders::mesh_frag::ShadowMapDataStd140 {
                shadow_map_view_proj: shadow_map_render_view.view_proj().to_cols_array_2d(),
                shadow_map_light_dir: shadow_map_render_view.view_dir().into(),
                ..Default::default()
            };
            shadow_map_images[index] = Some(&self.shadow_map_data.shadow_map_images[index]);
        }

        // HACK: Placate vulkan validation for now
        if let Some(first) = self.shadow_map_data.shadow_map_images.first() {
            for index in self.shadow_map_data.shadow_map_images.len()..MAX_SHADOW_MAPS {
                shadow_map_images[index] = Some(first);
            }
        }

        //
        // Create per-view descriptors for all per-view descriptor layouts that are in our materials
        //
        for &view in views {
            let mut per_view_frag_data = self.create_per_view_frag_data(view);
            per_view_frag_data.shadow_map_count =
                self.shadow_map_data
                    .shadow_map_render_views
                    .len()
                    .min(per_view_frag_data.shadow_maps.len()) as u32;
            per_view_frag_data.shadow_maps = shadow_maps;

            if view.phase_is_relevant::<OpaqueRenderPhase>()
                || view.phase_is_relevant::<ShadowMapRenderPhase>()
            {
                for per_view_descriptor_set_layout in &opaque_per_view_descriptor_set_layouts {
                    let descriptor_set = descriptor_set_allocator
                        .create_descriptor_set(
                            &per_view_descriptor_set_layout,
                            shaders::mesh_frag::DescriptorSet0Args {
                                shadow_map_images: &shadow_map_images,
                                per_view_data: &per_view_frag_data,
                            },
                        )
                        .unwrap();

                    let old = per_view_descriptor_sets.insert(
                        (view.view_index(), per_view_descriptor_set_layout.clone()),
                        descriptor_set,
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

                        let per_object_param = MeshPerObjectFragmentShaderParam {
                            model: extracted_data.world_transform.to_cols_array_2d(),
                            model_view: model_view.to_cols_array_2d(),
                            model_view_proj: model_view_proj.to_cols_array_2d(),
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
                                        Some(mesh_part.opaque_material_descriptor_set.clone()),
                                        false,
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
                                            None,
                                            true,
                                        );

                                        view_submit_nodes.add_submit_node::<ShadowMapRenderPhase>(
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

        per_view_data.ambient_light = glam::Vec4::new(0.03, 0.03, 0.03, 1.0).into();

        for light in &self.directional_lights {
            let light_count = per_view_data.directional_light_count as usize;
            if light_count > per_view_data.directional_lights.len() {
                break;
            }

            let light_from = glam::Vec3::zero();
            let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
            let light_to = light.light.direction;
            let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

            let light_direction = (light_to - light_from).normalize();
            let light_direction_vs = (light_to_vs - light_from_vs).normalize();

            let out = &mut per_view_data.directional_lights[light_count];
            out.direction_ws = light_direction.into();
            out.direction_vs = light_direction_vs.into();
            out.color = light.light.color.into();
            out.intensity = light.light.intensity;
            out.shadow_map = light.shadow_map_index.map(|x| x as i32).unwrap_or(-1);

            per_view_data.directional_light_count += 1;
        }

        for light in &self.point_lights {
            let light_count = per_view_data.point_light_count as usize;
            if light_count > per_view_data.point_lights.len() {
                break;
            }

            let out = &mut per_view_data.point_lights[light_count];
            out.position_ws = light.position.position.into();
            out.position_vs = (view.view_matrix() * light.position.position.extend(1.0))
                .truncate()
                .into();
            out.color = light.light.color.into();
            out.range = light.light.range;
            out.intensity = light.light.intensity;
            out.shadow_map = light.shadow_map_index.map(|x| x as i32).unwrap_or(-1);

            per_view_data.point_light_count += 1;
        }

        for light in &self.spot_lights {
            let light_count = per_view_data.spot_light_count as usize;
            if light_count > per_view_data.spot_lights.len() {
                break;
            }

            let light_from = light.position.position;
            let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
            let light_to = light.position.position + light.light.direction;
            let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

            let light_direction = (light_to - light_from).normalize();
            let light_direction_vs = (light_to_vs - light_from_vs).normalize();

            let out = &mut per_view_data.spot_lights[light_count];
            out.position_ws = light_from.into();
            out.position_vs = light_from_vs.into();
            out.direction_ws = light_direction.into();
            out.direction_vs = light_direction_vs.into();
            out.spotlight_half_angle = light.light.spotlight_half_angle;
            out.color = light.light.color.into();
            out.range = light.light.range;
            out.intensity = light.light.intensity;
            out.shadow_map = light.shadow_map_index.map(|x| x as i32).unwrap_or(-1);

            per_view_data.spot_light_count += 1;
        }

        per_view_data
    }

    fn add_render_node(
        descriptor_set_allocator: &mut DescriptorSetAllocatorRef,
        prepared_submit_node_mesh_data: &mut Vec<PreparedSubmitNodeMeshData>,
        per_view_descriptor_sets: &FnvHashMap<
            (u32, ResourceArc<DescriptorSetLayoutResource>),
            DescriptorSetArc,
        >,
        view: &RenderView,
        view_node: &PerViewNode,
        per_object_param: &MeshPerObjectFragmentShaderParam,
        mesh_part_index: usize,
        material_pass: &MaterialPass,
        per_material_descriptor_set: Option<DescriptorSetArc>,
        is_shadow_pass: bool,
    ) -> usize {
        let per_view_descriptor_set = if !is_shadow_pass {
            let per_view_descriptor_set_layout = material_pass.descriptor_set_layouts
                [super::PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                .clone();

            Some(
                per_view_descriptor_sets[&(view.view_index(), per_view_descriptor_set_layout)]
                    .clone(),
            )
        } else {
            None
        };

        //
        // Create the per-instance descriptor set
        // TODO: Common case is that parts in the same mesh use same material, so only create new descriptor set if the material is
        // different between parts.
        //
        let per_instance_descriptor_set_layout = &material_pass.descriptor_set_layouts
            [super::PER_INSTANCE_DESCRIPTOR_SET_INDEX as usize];

        let per_instance_descriptor_set = descriptor_set_allocator
            .create_descriptor_set(
                per_instance_descriptor_set_layout,
                shaders::mesh_frag::DescriptorSet2Args {
                    per_object_data: &per_object_param,
                },
            )
            .unwrap();
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
