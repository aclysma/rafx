use super::MeshCommandWriter;
use crate::components::{
    DirectionalLightComponent, PointLightComponent, PositionComponent, SpotLightComponent,
};
use crate::features::mesh::shadow_map_resource::ShadowMapResource;
use crate::features::mesh::{
    ExtractedDirectionalLight, ExtractedFrameNodeMeshData, ExtractedPointLight, ExtractedSpotLight,
    LightId, MeshPerObjectFragmentShaderParam, MeshPerViewFragmentShaderParam, MeshRenderFeature,
    PreparedSubmitNodeMeshData, ShadowMapRenderView, ShadowPerObjectShaderParam,
    ShadowPerViewShaderParam,
};
use crate::phases::{DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase};
use crate::StatsAllocMemoryRegion;
use fnv::{FnvHashMap, FnvHashSet};
use rafx::framework::MaterialPassResource;
use rafx::framework::{DescriptorSetArc, DescriptorSetLayoutResource, ResourceArc};
use rafx::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PerViewNode, PrepareJob, RenderFeature,
    RenderFeatureIndex, RenderJobPrepareContext, RenderView, RenderViewIndex, ViewSubmitNodes,
};
use rafx::renderer::InvalidResources;

pub struct PreparedDirectionalLight<'a> {
    light: &'a DirectionalLightComponent,
    shadow_map_index: Option<usize>,
}

pub struct PreparedPointLight<'a> {
    light: &'a PointLightComponent,
    position: &'a PositionComponent,
    shadow_map_index: Option<usize>,
}

pub struct PreparedSpotLight<'a> {
    light: &'a SpotLightComponent,
    position: &'a PositionComponent,
    shadow_map_index: Option<usize>,
}

pub struct MeshPrepareJob {
    pub(super) depth_material: ResourceArc<MaterialPassResource>,
    pub(super) extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub(super) directional_lights: Vec<ExtractedDirectionalLight>,
    pub(super) point_lights: Vec<ExtractedPointLight>,
    pub(super) spot_lights: Vec<ExtractedSpotLight>,
}

impl PrepareJob for MeshPrepareJob {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes) {
        profiling::scope!("Mesh Prepare");
        let invalid_resources = prepare_context.render_resources.fetch::<InvalidResources>();

        let shadow_map_data = prepare_context
            .render_resources
            .fetch::<ShadowMapResource>();

        let depth_material = &self.depth_material;

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        //TODO: reserve sizes
        let mut opaque_per_view_descriptor_set_layouts =
            FnvHashSet::<ResourceArc<DescriptorSetLayoutResource>>::default();
        let mut depth_per_view_descriptor_set_layouts =
            FnvHashSet::<ResourceArc<DescriptorSetLayoutResource>>::default();
        let mut prepared_submit_node_mesh_data = Vec::<PreparedSubmitNodeMeshData>::default();
        let mut per_view_descriptor_sets = FnvHashMap::<
            (RenderViewIndex, ResourceArc<DescriptorSetLayoutResource>),
            DescriptorSetArc,
        >::default();

        depth_per_view_descriptor_set_layouts.insert(
            depth_material.get_raw().descriptor_set_layouts
                [super::PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                .clone(),
        );

        //
        // Iterate every mesh part to find all the per-view descriptor sets layouts
        //
        // Realistically all the materials (for now) have identical layouts so iterating though them
        // like this just avoids hardcoding to find the first mesh's first opaque pass
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        {
            profiling::scope!("lookup per view descriptor set layouts");
            for mesh in &self.extracted_frame_node_mesh_data {
                if let Some(mesh) = mesh {
                    for mesh_part in &*mesh.mesh_asset.inner.mesh_parts {
                        if let Some(mesh_part) = mesh_part {
                            opaque_per_view_descriptor_set_layouts.insert(
                                mesh_part
                                    .opaque_pass
                                    .material_pass_resource
                                    .get_raw()
                                    .descriptor_set_layouts
                                    [super::PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                                    .clone(),
                            );
                        }
                    }
                }
            }
        }

        //
        // Create uniform data for each shadow map and a properly-sized static array of the images.
        // This will take our mixed list of shadow maps and separate them into 2d (spot and
        // directional lights) and cube (point lights)
        //
        //TODO: Pull this const from the shader
        const MAX_SHADOW_MAPS_2D: usize = 32;
        const MAX_SHADOW_MAPS_CUBE: usize = 16;

        let mut shadow_map_2d_count = 0;
        let mut shadow_map_2d_data =
            [shaders::mesh_frag::ShadowMap2DDataStd140::default(); MAX_SHADOW_MAPS_2D];
        let mut shadow_map_2d_image_views = [None; MAX_SHADOW_MAPS_2D];

        let mut shadow_map_cube_count = 0;
        let mut shadow_map_cube_data =
            [shaders::mesh_frag::ShadowMapCubeDataStd140::default(); MAX_SHADOW_MAPS_CUBE];
        let mut shadow_map_cube_image_views = [None; MAX_SHADOW_MAPS_CUBE];

        // This maps the index in the combined list to indices in the 2d/cube maps
        let mut image_index_remap = vec![None; shadow_map_data.shadow_map_image_views.len()];

        assert_eq!(
            shadow_map_data.shadow_map_render_views.len(),
            shadow_map_data.shadow_map_image_views.len()
        );

        {
            profiling::scope!("gather shadow data");

            for (index, shadow_map_render_view) in
                shadow_map_data.shadow_map_render_views.iter().enumerate()
            {
                match shadow_map_render_view {
                    ShadowMapRenderView::Single(view) => {
                        if shadow_map_2d_count >= MAX_SHADOW_MAPS_2D {
                            log::warn!("More 2D shadow maps than the mesh shader can support");
                            continue;
                        }

                        shadow_map_2d_data[shadow_map_2d_count] =
                            shaders::mesh_frag::ShadowMap2DDataStd140 {
                                shadow_map_view_proj: view.view_proj().to_cols_array_2d(),
                                shadow_map_light_dir: view.view_dir().into(),
                                ..Default::default()
                            };

                        shadow_map_2d_image_views[shadow_map_2d_count] =
                            Some(&shadow_map_data.shadow_map_image_views[index]);
                        image_index_remap[index] = Some(shadow_map_2d_count);
                        shadow_map_2d_count += 1;
                    }
                    ShadowMapRenderView::Cube(views) => {
                        if shadow_map_cube_count >= MAX_SHADOW_MAPS_CUBE {
                            log::warn!("More cube shadow maps than the mesh shader can support");
                            continue;
                        }

                        // Shader not set up for infinite far plane
                        let (near, far) = views[0]
                            .depth_range()
                            .finite_planes_after_reverse()
                            .unwrap();
                        shadow_map_cube_data[shadow_map_cube_count] =
                            shaders::mesh_frag::ShadowMapCubeDataStd140 {
                                cube_map_projection_near_z: near,
                                cube_map_projection_far_z: far,
                                ..Default::default()
                            };

                        shadow_map_cube_image_views[shadow_map_cube_count] =
                            Some(&shadow_map_data.shadow_map_image_views[index]);
                        image_index_remap[index] = Some(shadow_map_cube_count);
                        shadow_map_cube_count += 1;
                    }
                }
            }

            for index in shadow_map_2d_count..MAX_SHADOW_MAPS_2D {
                shadow_map_2d_image_views[index] = Some(&invalid_resources.invalid_image);
            }

            for index in shadow_map_cube_count..MAX_SHADOW_MAPS_CUBE {
                shadow_map_cube_image_views[index] =
                    Some(&invalid_resources.invalid_cube_map_image);
            }
        }

        //
        // Assign all direction lights a shadow map slot
        //
        let mut prepared_directional_lights = Vec::with_capacity(self.directional_lights.len());
        for directional_light in &self.directional_lights {
            prepared_directional_lights.push(PreparedDirectionalLight {
                light: &directional_light.light,
                shadow_map_index: shadow_map_data
                    .shadow_map_lookup
                    .get(&LightId::DirectionalLight(directional_light.entity))
                    .map(|x| image_index_remap[*x])
                    .flatten(),
            });
        }

        //
        // Assign all spot lights a shadow map slot
        //
        let mut prepared_spot_lights = Vec::with_capacity(self.spot_lights.len());
        for spot_light in &self.spot_lights {
            prepared_spot_lights.push(PreparedSpotLight {
                light: &spot_light.light,
                position: &spot_light.position,
                shadow_map_index: shadow_map_data
                    .shadow_map_lookup
                    .get(&LightId::SpotLight(spot_light.entity))
                    .map(|x| image_index_remap[*x])
                    .flatten(),
            });
        }

        //
        // Assign all point lights a CUBE shadow map slot
        //
        let mut prepared_point_lights = Vec::with_capacity(self.point_lights.len());
        for point_light in &self.point_lights {
            prepared_point_lights.push(PreparedPointLight {
                light: &point_light.light,
                position: &point_light.position,
                shadow_map_index: shadow_map_data
                    .shadow_map_lookup
                    .get(&LightId::PointLight(point_light.entity))
                    .map(|x| image_index_remap[*x])
                    .flatten(),
            });
        }

        //
        // Create per-view descriptors for all per-view descriptor layouts that are in our materials
        //
        {
            profiling::scope!("create per view descriptor sets");
            for view in views {
                if view.phase_is_relevant::<DepthPrepassRenderPhase>()
                    || view.phase_is_relevant::<ShadowMapRenderPhase>()
                {
                    let mut per_view_data = ShadowPerViewShaderParam::default();

                    per_view_data.view = view.view_matrix().to_cols_array_2d();
                    per_view_data.view_proj = view.view_proj().to_cols_array_2d();

                    for per_view_descriptor_set_layout in &depth_per_view_descriptor_set_layouts {
                        let descriptor_set = descriptor_set_allocator
                            .create_descriptor_set(
                                &per_view_descriptor_set_layout,
                                shaders::depth_vert::DescriptorSet0Args {
                                    per_view_data: &per_view_data,
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

                if view.phase_is_relevant::<OpaqueRenderPhase>() {
                    let mut per_view_frag_data = self.create_per_view_frag_data(
                        view,
                        &prepared_directional_lights,
                        &prepared_spot_lights,
                        &prepared_point_lights,
                    );

                    per_view_frag_data.shadow_map_2d_data = shadow_map_2d_data;
                    per_view_frag_data.shadow_map_cube_data = shadow_map_cube_data;

                    for per_view_descriptor_set_layout in &opaque_per_view_descriptor_set_layouts {
                        let descriptor_set = descriptor_set_allocator
                            .create_descriptor_set(
                                &per_view_descriptor_set_layout,
                                shaders::mesh_frag::DescriptorSet0Args {
                                    shadow_map_images: &shadow_map_2d_image_views,
                                    shadow_map_images_cube: &shadow_map_cube_image_views,
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
        }

        let mut opaque_frame_node_per_instance_descriptor_sets =
            vec![None; self.extracted_frame_node_mesh_data.len()];
        let mut shadow_map_frame_node_per_instance_descriptor_sets =
            vec![None; self.extracted_frame_node_mesh_data.len()];
        {
            profiling::scope!("create per instance descriptor sets");
            StatsAllocMemoryRegion::new("create per instance descriptor sets");

            for (frame_node_index, frame_node_data) in
                self.extracted_frame_node_mesh_data.iter().enumerate()
            {
                if let Some(frame_node_data) = frame_node_data {
                    let model = frame_node_data.world_transform.to_cols_array_2d();

                    for mesh_part in frame_node_data.mesh_asset.inner.mesh_parts.iter() {
                        if let Some(mesh_part) = mesh_part {
                            {
                                let per_object_data = MeshPerObjectFragmentShaderParam { model };

                                let per_instance_descriptor_set_layout = &mesh_part
                                    .opaque_pass
                                    .material_pass_resource
                                    .get_raw()
                                    .descriptor_set_layouts
                                    [super::PER_INSTANCE_DESCRIPTOR_SET_INDEX as usize];

                                opaque_frame_node_per_instance_descriptor_sets[frame_node_index] =
                                    Some(
                                        descriptor_set_allocator
                                            .create_descriptor_set_with_writer(
                                                per_instance_descriptor_set_layout,
                                                shaders::mesh_frag::DescriptorSet2Args {
                                                    per_object_data: &per_object_data,
                                                },
                                            )
                                            .unwrap(),
                                    );
                            }

                            {
                                let per_object_data = ShadowPerObjectShaderParam { model };

                                let per_instance_descriptor_set_layout =
                                    &depth_material.get_raw().descriptor_set_layouts
                                        [super::PER_INSTANCE_DESCRIPTOR_SET_INDEX as usize];

                                shadow_map_frame_node_per_instance_descriptor_sets
                                    [frame_node_index] = Some(
                                    descriptor_set_allocator
                                        .create_descriptor_set_with_writer(
                                            per_instance_descriptor_set_layout,
                                            shaders::depth_vert::DescriptorSet2Args {
                                                per_object_data: &per_object_data,
                                            },
                                        )
                                        .unwrap(),
                                );
                            }
                        }
                    }
                }
            }
        }

        //
        // Produce render nodes for every mesh
        //
        for view in views {
            profiling::scope!("create render nodes for view");
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());

            let view_nodes = frame_packet.view_nodes(view, self.feature_index());
            if let Some(view_nodes) = view_nodes {
                for view_node in view_nodes {
                    let extracted_data =
                        &self.extracted_frame_node_mesh_data[view_node.frame_node_index() as usize];

                    if let Some(extracted_data) = extracted_data {
                        //let per_object_descriptor = frame_node_descriptor_sets[view_node.frame_node_index() as usize].as_ref().unwrap().clone();

                        let world_position = extracted_data.world_transform.w_axis().truncate();
                        let distance = (view.eye_position() - world_position).length_squared();

                        for (mesh_part_index, mesh_part) in extracted_data
                            .mesh_asset
                            .inner
                            .mesh_parts
                            .iter()
                            .enumerate()
                        {
                            if let Some(mesh_part) = mesh_part {
                                //
                                // Depth prepass for opaque objects
                                //
                                if view.phase_is_relevant::<DepthPrepassRenderPhase>() {
                                    let per_object_descriptor =
                                        shadow_map_frame_node_per_instance_descriptor_sets
                                            [view_node.frame_node_index() as usize]
                                            .as_ref()
                                            .unwrap();

                                    let depth_prepass_submit_node_index =
                                        MeshPrepareJob::add_render_node(
                                            &mut prepared_submit_node_mesh_data,
                                            &per_view_descriptor_sets,
                                            &view,
                                            view_node,
                                            per_object_descriptor,
                                            mesh_part_index,
                                            depth_material.clone(),
                                            None,
                                        );

                                    view_submit_nodes.add_submit_node::<DepthPrepassRenderPhase>(
                                        depth_prepass_submit_node_index as u32,
                                        0,
                                        distance,
                                    );
                                }

                                //
                                // Write opaque render node, if it's relevant
                                //
                                if view.phase_is_relevant::<OpaqueRenderPhase>() {
                                    let per_object_descriptor =
                                        opaque_frame_node_per_instance_descriptor_sets
                                            [view_node.frame_node_index() as usize]
                                            .as_ref()
                                            .unwrap();

                                    let opaque_submit_node_index = MeshPrepareJob::add_render_node(
                                        &mut prepared_submit_node_mesh_data,
                                        &per_view_descriptor_sets,
                                        &view,
                                        view_node,
                                        per_object_descriptor,
                                        mesh_part_index,
                                        mesh_part.opaque_pass.material_pass_resource.clone(),
                                        Some(mesh_part.opaque_material_descriptor_set.clone()),
                                    );

                                    view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(
                                        opaque_submit_node_index as u32,
                                        0,
                                        distance,
                                    );
                                }

                                //
                                // Write shadow map render node, if it's relevant
                                //
                                let casts_shadows = true; // TODO(dvd): Make this configurable somehow.
                                if casts_shadows {
                                    if view.phase_is_relevant::<ShadowMapRenderPhase>() {
                                        let per_object_descriptor =
                                            shadow_map_frame_node_per_instance_descriptor_sets
                                                [view_node.frame_node_index() as usize]
                                                .as_ref()
                                                .unwrap();

                                        let submit_node_index = MeshPrepareJob::add_render_node(
                                            &mut prepared_submit_node_mesh_data,
                                            &per_view_descriptor_sets,
                                            &view,
                                            view_node,
                                            per_object_descriptor,
                                            mesh_part_index,
                                            depth_material.clone(),
                                            None,
                                        );

                                        view_submit_nodes.add_submit_node::<ShadowMapRenderPhase>(
                                            submit_node_index as u32,
                                            0,
                                            distance,
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
        directional_lights: &[PreparedDirectionalLight],
        spot_lights: &[PreparedSpotLight],
        point_lights: &[PreparedPointLight],
    ) -> MeshPerViewFragmentShaderParam {
        let mut per_view_data = MeshPerViewFragmentShaderParam::default();

        per_view_data.view = view.view_matrix().to_cols_array_2d();
        per_view_data.view_proj = view.view_proj().to_cols_array_2d();

        per_view_data.ambient_light = glam::Vec4::new(0.03, 0.03, 0.03, 1.0).into();

        for light in directional_lights {
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

        for light in point_lights {
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

        for light in spot_lights {
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
        prepared_submit_node_mesh_data: &mut Vec<PreparedSubmitNodeMeshData>,
        per_view_descriptor_sets: &FnvHashMap<
            (u32, ResourceArc<DescriptorSetLayoutResource>),
            DescriptorSetArc,
        >,
        view: &RenderView,
        view_node: &PerViewNode,
        per_object_descriptor: &DescriptorSetArc,
        mesh_part_index: usize,
        material_pass_resource: ResourceArc<MaterialPassResource>,
        per_material_descriptor_set: Option<DescriptorSetArc>,
    ) -> usize {
        let per_view_descriptor_set = {
            let per_view_descriptor_set_layout = material_pass_resource
                .get_raw()
                .descriptor_set_layouts[super::PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                .clone();

            per_view_descriptor_sets[&(view.view_index(), per_view_descriptor_set_layout)].clone()
        };

        //
        // Create the submit node
        //
        let submit_node_index = prepared_submit_node_mesh_data.len();
        prepared_submit_node_mesh_data.push(PreparedSubmitNodeMeshData {
            material_pass_resource: material_pass_resource.clone(),
            per_view_descriptor_set,
            per_material_descriptor_set: per_material_descriptor_set.clone(),
            per_instance_descriptor_set: per_object_descriptor.clone(),
            frame_node_index: view_node.frame_node_index(),
            mesh_part_index,
        });
        submit_node_index
    }
}
