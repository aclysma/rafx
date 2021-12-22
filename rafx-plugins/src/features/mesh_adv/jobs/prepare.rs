use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase, WireframeRenderPhase,
};
use rafx::base::resource_map::ReadBorrow;
use rafx::framework::{MaterialPassResource, ResourceArc, ResourceContext};

use crate::shaders::mesh_adv::mesh_adv_textured_frag;
use glam::Mat4;
use rafx::api::{RafxBufferDef, RafxDeviceContext, RafxMemoryUsage, RafxResourceType};

use crate::shaders::depth::depth_vert;
use crate::shaders::mesh_adv::shadow_atlas_depth_vert;
use mesh_adv_textured_frag::PerViewDataUniform as MeshPerViewFragmentShaderParam;

const PER_VIEW_DESCRIPTOR_SET_INDEX: u32 =
    mesh_adv_textured_frag::PER_VIEW_DATA_DESCRIPTOR_SET_INDEX as u32;

pub struct MeshBasicPrepareJob<'prepare> {
    resource_context: ResourceContext,
    device_context: RafxDeviceContext,
    #[allow(dead_code)]
    requires_textured_descriptor_sets: bool,
    #[allow(dead_code)]
    requires_untextured_descriptor_sets: bool,
    depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    shadow_map_atlas_depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    shadow_map_data: ReadBorrow<'prepare, MeshBasicShadowMapResource>,
    render_object_instance_transforms: Arc<AtomicOnceCellStack<[[f32; 4]; 4]>>,
    #[allow(dead_code)]
    render_objects: MeshBasicRenderObjectSet,
}

impl<'prepare> MeshBasicPrepareJob<'prepare> {
    pub fn new(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<MeshBasicFramePacket>,
        submit_packet: Box<MeshSubmitPacket>,
        render_objects: MeshBasicRenderObjectSet,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        let mut requires_textured_descriptor_sets = false;
        let mut requires_untextured_descriptor_sets = false;

        for view in frame_packet.view_packets() {
            if view
                .view()
                .feature_flag_is_relevant::<MeshBasicUntexturedRenderFeatureFlag>()
            {
                requires_untextured_descriptor_sets = true;
            } else {
                requires_textured_descriptor_sets = true;
            }
        }

        let per_frame_data = frame_packet.per_frame_data().get();
        Arc::new(PrepareJob::new(
            Self {
                resource_context: prepare_context.resource_context.clone(),
                device_context: prepare_context.device_context.clone(),
                render_object_instance_transforms: {
                    // TODO: Ideally this would use an allocator from the `prepare_context`.
                    Arc::new(AtomicOnceCellStack::with_capacity(
                        frame_packet.render_object_instances().len(),
                    ))
                },
                depth_material_pass: { per_frame_data.depth_material_pass.clone() },
                shadow_map_atlas_depth_material_pass: {
                    per_frame_data.shadow_map_atlas_depth_material_pass.clone()
                },
                shadow_map_data: {
                    prepare_context
                        .render_resources
                        .fetch::<MeshBasicShadowMapResource>()
                },
                requires_textured_descriptor_sets,
                requires_untextured_descriptor_sets,
                render_objects,
            },
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for MeshBasicPrepareJob<'prepare> {
    fn begin_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        let mut per_frame_submit_data = Box::new(MeshBasicPerFrameSubmitData {
            num_shadow_map_2d: 0,
            shadow_map_2d_data: Default::default(),
            num_shadow_map_cube: 0,
            shadow_map_cube_data: Default::default(),
            shadow_map_image_index_remap: Default::default(),
            model_matrix_buffer: Default::default(),
        });

        let shadow_map_data = &self.shadow_map_data;

        //
        // Create uniform data for each shadow map and a properly-sized static array of the images.
        // This will take our mixed list of shadow maps and separate them into 2d (spot and
        // directional lights) and cube (point lights)
        //

        // This maps the index in the combined list to indices in the 2d/cube maps

        assert_eq!(
            shadow_map_data.shadow_map_render_views.len(),
            shadow_map_data.shadow_map_atlas_element_assignments.len()
        );

        {
            profiling::scope!("gather shadow data");

            for (_light_id, shadow_view_indices) in
                shadow_map_data.shadow_map_lookup_by_light_id.iter()
            {
                match shadow_view_indices {
                    MeshBasicShadowMapRenderViewIndices::Single(shadow_view_index) => {
                        let shadow_assignment =
                            shadow_map_data.shadow_map_atlas_element_assignment(*shadow_view_index);
                        let shadow_view =
                            shadow_map_data.shadow_map_render_views_meta(*shadow_view_index);

                        let num_shadow_map_2d = per_frame_submit_data.num_shadow_map_2d;
                        if num_shadow_map_2d >= MAX_SHADOW_MAPS_2D {
                            log::warn!("More 2D shadow maps than the mesh shader can support");
                            continue;
                        }

                        let shadow_info = shadow_assignment.info();
                        per_frame_submit_data.shadow_map_2d_data[num_shadow_map_2d] =
                            mesh_adv_textured_frag::ShadowMap2DDataStd140 {
                                uv_min: shadow_info.uv_min.into(),
                                uv_max: shadow_info.uv_max.into(),
                                shadow_map_view_proj: shadow_view.view_proj.to_cols_array_2d(),
                                shadow_map_light_dir: shadow_view.view_dir.into(),
                                ..Default::default()
                            };

                        let old = per_frame_submit_data
                            .shadow_map_image_index_remap
                            .insert(*shadow_view_index, num_shadow_map_2d);
                        assert!(old.is_none());

                        per_frame_submit_data.num_shadow_map_2d += 1;
                    }
                    MeshBasicShadowMapRenderViewIndices::Cube(shadow_views) => {
                        let num_shadow_map_cube = per_frame_submit_data.num_shadow_map_cube;
                        if num_shadow_map_cube >= MAX_SHADOW_MAPS_CUBE {
                            log::warn!("More cube shadow maps than the mesh shader can support");
                            continue;
                        }

                        let mut near = 0.0;
                        let mut far = 1.0;

                        let mut uv_min_uv_max = [[-1.0, -1.0, -1.0, -1.0]; 6];
                        for (i, shadow_view_index) in shadow_views.iter().enumerate() {
                            if let Some(shadow_view_index) = shadow_view_index {
                                let shadow_assignment = shadow_map_data
                                    .shadow_map_atlas_element_assignment(*shadow_view_index);
                                let shadow_view = shadow_map_data
                                    .shadow_map_render_views_meta(*shadow_view_index);

                                let shadow_info = shadow_assignment.info();
                                uv_min_uv_max[i] = [
                                    shadow_info.uv_min[0],
                                    shadow_info.uv_min[1],
                                    shadow_info.uv_max[0],
                                    shadow_info.uv_max[1],
                                ];

                                let near_far = shadow_view
                                    .depth_range
                                    .finite_planes_after_reverse()
                                    .unwrap();
                                near = near_far.0;
                                far = near_far.1;

                                let old = per_frame_submit_data
                                    .shadow_map_image_index_remap
                                    .insert(*shadow_view_index, num_shadow_map_cube);
                                assert!(old.is_none());
                            }
                        }

                        per_frame_submit_data.shadow_map_cube_data[num_shadow_map_cube] =
                            mesh_adv_textured_frag::ShadowMapCubeDataStd140 {
                                uv_min_uv_max,
                                cube_map_projection_near_z: near,
                                cube_map_projection_far_z: far,
                                ..Default::default()
                            };

                        per_frame_submit_data.num_shadow_map_cube += 1;
                    }
                }
            }
        }

        context
            .submit_packet()
            .per_frame_submit_data()
            .set(per_frame_submit_data);
    }

    fn prepare_render_object_instance(
        &self,
        _job_context: &mut DefaultJobContext,
        context: &PrepareRenderObjectInstanceContext<'prepare, '_, Self>,
    ) {
        let render_object_instance = context.render_object_instance_data();
        if render_object_instance.is_none() {
            return;
        }

        let extracted_data = render_object_instance.as_ref().unwrap();
        let world_transform = Mat4::from_scale_rotation_translation(
            extracted_data.scale,
            extracted_data.rotation,
            extracted_data.translation,
        );

        let model = world_transform.to_cols_array_2d();
        let model_matrix_offset = self.render_object_instance_transforms.push(model);

        context.set_render_object_instance_submit_data(MeshBasicRenderObjectInstanceSubmitData {
            model_matrix_offset,
        });
    }

    fn prepare_render_object_instance_per_view(
        &self,
        _job_context: &mut DefaultJobContext,
        context: &PrepareRenderObjectInstancePerViewContext<'prepare, '_, Self>,
    ) {
        let view = context.view();

        if let Some(extracted_data) = context.render_object_instance_data() {
            let distance = (view.eye_position() - extracted_data.translation).length_squared();
            let render_object_instance_id = context.render_object_instance_id();

            let model_matrix_offset = context
                .render_object_instance_submit_data()
                .model_matrix_offset;

            for (mesh_part_index, mesh_part) in extracted_data
                .mesh_asset
                .inner
                .mesh_parts
                .iter()
                .enumerate()
            {
                if mesh_part.is_none() {
                    continue;
                }

                let mesh_part = mesh_part.as_ref().unwrap();

                if view.phase_is_relevant::<DepthPrepassRenderPhase>() {
                    context.push_submit_node::<DepthPrepassRenderPhase>(
                        MeshBasicDrawCall {
                            render_object_instance_id,
                            material_pass_resource: self
                                .depth_material_pass
                                .as_ref()
                                .unwrap()
                                .clone(),
                            per_material_descriptor_set: None,
                            mesh_part_index,
                            model_matrix_offset,
                        },
                        0,
                        distance,
                    );
                }

                if view.phase_is_relevant::<ShadowMapRenderPhase>() {
                    context.push_submit_node::<ShadowMapRenderPhase>(
                        MeshBasicDrawCall {
                            render_object_instance_id,
                            material_pass_resource: self
                                .shadow_map_atlas_depth_material_pass
                                .as_ref()
                                .unwrap()
                                .clone(),
                            per_material_descriptor_set: None,
                            mesh_part_index,
                            model_matrix_offset,
                        },
                        0,
                        distance,
                    );
                }

                if view.phase_is_relevant::<OpaqueRenderPhase>() {
                    let material_pass_resource = mesh_part
                        .get_material_pass_resource(view, OpaqueRenderPhase::render_phase_index())
                        .clone();

                    let per_material_descriptor_set = Some(
                        mesh_part
                            .get_material_descriptor_set(
                                view,
                                OpaqueRenderPhase::render_phase_index(),
                            )
                            .clone(),
                    );

                    context.push_submit_node::<OpaqueRenderPhase>(
                        MeshBasicDrawCall {
                            render_object_instance_id,
                            material_pass_resource,
                            per_material_descriptor_set,
                            mesh_part_index,
                            model_matrix_offset,
                        },
                        0,
                        distance,
                    );
                }

                if view.phase_is_relevant::<WireframeRenderPhase>()
                    && view.feature_flag_is_relevant::<MeshBasicWireframeRenderFeatureFlag>()
                {
                    let material_pass_resource = mesh_part
                        .get_material_pass_resource(
                            view,
                            WireframeRenderPhase::render_phase_index(),
                        )
                        .clone();

                    let per_material_descriptor_set = Some(
                        mesh_part
                            .get_material_descriptor_set(
                                view,
                                OpaqueRenderPhase::render_phase_index(),
                            )
                            .clone(),
                    );

                    context.push_submit_node::<WireframeRenderPhase>(
                        MeshBasicDrawCall {
                            render_object_instance_id,
                            material_pass_resource,
                            per_material_descriptor_set,
                            mesh_part_index,
                            model_matrix_offset,
                        },
                        0,
                        distance,
                    );
                }
            }
        }
    }

    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();
        let shadow_map_data = &self.shadow_map_data;

        let per_view_data = context.per_view_data();
        let per_frame_submit_data = context.per_frame_submit_data();

        let view = context.view();
        let is_lit = !view.feature_flag_is_relevant::<MeshBasicUnlitRenderFeatureFlag>();
        let has_shadows = !view.feature_flag_is_relevant::<MeshBasicNoShadowsRenderFeatureFlag>();

        let opaque_descriptor_set = if view.phase_is_relevant::<OpaqueRenderPhase>() {
            let per_view_frag_data = {
                let mut per_view_frag_data = MeshPerViewFragmentShaderParam::default();

                per_view_frag_data.view = view.view_matrix().to_cols_array_2d();
                per_view_frag_data.view_proj = view.view_proj().to_cols_array_2d();
                per_view_frag_data.ambient_light = if is_lit {
                    per_view_data.ambient_light.extend(1.0).into()
                } else {
                    glam::Vec4::ONE.into()
                };

                for directional_light in &per_view_data.directional_lights {
                    if directional_light.is_none() {
                        break;
                    }

                    let light_count = per_view_frag_data.directional_light_count as usize;
                    if light_count > per_view_frag_data.directional_lights.len() {
                        break;
                    }

                    let directional_light = directional_light.as_ref().unwrap();
                    let light = directional_light;
                    let shadow_map_index = shadow_map_data
                        .shadow_map_lookup_by_light_id
                        .get(&MeshBasicLightId::DirectionalLight(
                            directional_light.object_id,
                        ))
                        .map(|x| {
                            per_frame_submit_data.shadow_map_image_index_remap[&x.unwrap_single()]
                        });

                    let light_from = glam::Vec3::ZERO;
                    let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
                    let light_to = light.light.direction;
                    let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

                    let light_direction = (light_to - light_from).normalize();
                    let light_direction_vs = (light_to_vs - light_from_vs).normalize();

                    let out = &mut per_view_frag_data.directional_lights[light_count];
                    out.direction_ws = light_direction.into();
                    out.direction_vs = light_direction_vs.into();
                    out.color = light.light.color.into();
                    out.intensity = light.light.intensity;
                    out.shadow_map = if has_shadows {
                        shadow_map_index.map(|x| x as i32).unwrap_or(-1)
                    } else {
                        -1
                    };

                    per_view_frag_data.directional_light_count += 1;
                }

                for point_light in &per_view_data.point_lights {
                    if point_light.is_none() {
                        break;
                    }

                    let light_count = per_view_frag_data.point_light_count as usize;
                    if light_count > per_view_frag_data.point_lights.len() {
                        break;
                    }

                    let point_light = point_light.as_ref().unwrap();
                    let light = point_light;
                    let shadow_map_index = shadow_map_data
                        .shadow_map_lookup_by_light_id
                        .get(&MeshBasicLightId::PointLight(point_light.object_id))
                        .map(|x| {
                            per_frame_submit_data.shadow_map_image_index_remap[&x.unwrap_cube_any()]
                        });

                    let out = &mut per_view_frag_data.point_lights[light_count];
                    out.position_ws = light.transform.translation.into();
                    out.position_vs = (view.view_matrix()
                        * light.transform.translation.extend(1.0))
                    .truncate()
                    .into();
                    out.color = light.light.color.into();
                    out.range = light.light.range;
                    out.intensity = light.light.intensity;
                    out.shadow_map = if has_shadows {
                        shadow_map_index.map(|x| x as i32).unwrap_or(-1)
                    } else {
                        -1
                    };

                    per_view_frag_data.point_light_count += 1;
                }

                for spot_light in &per_view_data.spot_lights {
                    if spot_light.is_none() {
                        break;
                    }

                    let light_count = per_view_frag_data.spot_light_count as usize;
                    if light_count > per_view_frag_data.spot_lights.len() {
                        break;
                    }

                    let spot_light = spot_light.as_ref().unwrap();
                    let light = spot_light;
                    let shadow_map_index = shadow_map_data
                        .shadow_map_lookup_by_light_id
                        .get(&MeshBasicLightId::SpotLight(spot_light.object_id))
                        .map(|x| {
                            per_frame_submit_data.shadow_map_image_index_remap[&x.unwrap_single()]
                        });

                    let light_from = light.transform.translation;
                    let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();
                    let light_to = light.transform.translation + light.light.direction;
                    let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

                    let light_direction = (light_to - light_from).normalize();
                    let light_direction_vs = (light_to_vs - light_from_vs).normalize();

                    let out = &mut per_view_frag_data.spot_lights[light_count];
                    out.position_ws = light_from.into();
                    out.position_vs = light_from_vs.into();
                    out.direction_ws = light_direction.into();
                    out.direction_vs = light_direction_vs.into();
                    out.spotlight_half_angle = light.light.spotlight_half_angle;
                    out.color = light.light.color.into();
                    out.range = light.light.range;
                    out.intensity = light.light.intensity;
                    out.shadow_map = if has_shadows {
                        shadow_map_index.map(|x| x as i32).unwrap_or(-1)
                    } else {
                        -1
                    };

                    per_view_frag_data.spot_light_count += 1;
                }

                per_view_frag_data.shadow_map_2d_data = per_frame_submit_data.shadow_map_2d_data;
                per_view_frag_data.shadow_map_cube_data =
                    per_frame_submit_data.shadow_map_cube_data;

                per_view_frag_data
            };

            let shadow_map_atlas = context.per_frame_data().shadow_map_atlas.clone();

            // NOTE(dvd): This assumes that all opaque materials have the same per view descriptor set layout.
            let opaque_per_view_descriptor_set_layout = {
                let mut opaque_per_view_descriptor_set_layout = None;

                for id in 0..context.render_object_instances().len() {
                    // TODO(dvd): This could be replaced by an `iter` or `as_slice` method on the data.
                    if let Some(extracted_data) = context.render_object_instances_data().get(id) {
                        let mesh_parts = &extracted_data.mesh_asset.inner.mesh_parts;
                        for mesh_part in mesh_parts {
                            if let Some(mesh_part) = mesh_part {
                                opaque_per_view_descriptor_set_layout = Some(
                                    mesh_part
                                        .get_material_pass_resource(
                                            view,
                                            OpaqueRenderPhase::render_phase_index(),
                                        )
                                        .get_raw()
                                        .descriptor_set_layouts
                                        [PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                                        .clone(),
                                );
                                break;
                            }
                        }
                    }

                    if opaque_per_view_descriptor_set_layout.is_some() {
                        break;
                    }
                }

                opaque_per_view_descriptor_set_layout
            };

            opaque_per_view_descriptor_set_layout.as_ref().and_then(
                |per_view_descriptor_set_layout| {
                    descriptor_set_allocator
                        .create_descriptor_set(
                            &per_view_descriptor_set_layout,
                            mesh_adv_textured_frag::DescriptorSet0Args {
                                shadow_map_atlas: &shadow_map_atlas,
                                per_view_data: &per_view_frag_data,
                            },
                        )
                        .ok()
                },
            )
        } else {
            None
        };

        //
        // If we are rendering shadow maps to the shadow map atlas, make a descriptor set with uniform
        // data for this view
        //
        let shadow_map_atlas_depth_descriptor_set = {
            let atlas_info = self
                .shadow_map_data
                .shadow_map_atlas_element_info_for_view(view.view_index());

            if view.phase_is_relevant::<ShadowMapRenderPhase>() && atlas_info.is_some() {
                let mut per_view_data = shadow_atlas_depth_vert::PerViewDataUniform::default();

                let atlas_info = atlas_info.unwrap();

                per_view_data.view = view.view_matrix().to_cols_array_2d();
                per_view_data.view_proj = view.view_proj().to_cols_array_2d();
                per_view_data.uv_min = atlas_info.uv_min.into();
                per_view_data.uv_max = atlas_info.uv_max.into();

                // This is the equivalent code for doing a matrix transform to place the projection
                // in the correct place. Keeping this around for now for reference.
                // let uv_width = atlas_info.uv_max[0] - atlas_info.uv_min[0];
                // let uv_height = atlas_info.uv_max[1] - atlas_info.uv_min[1];
                // let x_translate = (atlas_info.uv_min[0] * 2.0 - 1.0) + (2.0 * (uv_width / 2.0));
                // let y_translate =
                //     ((1.0 - atlas_info.uv_min[1]) * 2.0 - 1.0) - (2.0 * (uv_height / 2.0));
                // let view_proj_atlassed =
                //     glam::Mat4::from_translation(glam::Vec3::new(x_translate, y_translate, 0.0))
                //         * glam::Mat4::from_scale(glam::Vec3::new(uv_width, uv_height, 1.0))
                //         * view.view_proj();
                //per_view_data.view_proj_atlassed = view_proj_atlassed.to_cols_array_2d();

                let per_instance_descriptor_set_layout = &self
                    .shadow_map_atlas_depth_material_pass
                    .as_ref()
                    .unwrap()
                    .get_raw()
                    .descriptor_set_layouts[PER_VIEW_DESCRIPTOR_SET_INDEX as usize];

                descriptor_set_allocator
                    .create_descriptor_set(
                        per_instance_descriptor_set_layout,
                        shadow_atlas_depth_vert::DescriptorSet0Args {
                            per_view_data: &per_view_data,
                        },
                    )
                    .ok()
            } else {
                None
            }
        };

        //
        // If we are rendering a depth prepass, make a scriptor set with uniform data for this view
        //
        let depth_descriptor_set = if view.phase_is_relevant::<DepthPrepassRenderPhase>() {
            let mut per_view_data = depth_vert::PerViewDataUniform::default();

            per_view_data.view = view.view_matrix().to_cols_array_2d();
            per_view_data.view_proj = view.view_proj().to_cols_array_2d();

            let per_instance_descriptor_set_layout = &self
                .depth_material_pass
                .as_ref()
                .unwrap()
                .get_raw()
                .descriptor_set_layouts[PER_VIEW_DESCRIPTOR_SET_INDEX as usize];

            descriptor_set_allocator
                .create_descriptor_set(
                    per_instance_descriptor_set_layout,
                    depth_vert::DescriptorSet0Args {
                        per_view_data: &per_view_data,
                    },
                )
                .ok()
        } else {
            None
        };

        context
            .view_submit_packet()
            .per_view_submit_data()
            .set(MeshBasicPerViewSubmitData {
                opaque_descriptor_set,
                depth_descriptor_set,
                shadow_map_atlas_depth_descriptor_set,
            });
    }

    fn end_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        let mut model_matrix_buffer = context
            .per_frame_submit_data()
            .model_matrix_buffer
            .borrow_mut();

        *model_matrix_buffer = if self.render_object_instance_transforms.len() > 0 {
            let dyn_resource_allocator_set =
                self.resource_context.create_dyn_resource_allocator_set();

            let vertex_buffer_size = self.render_object_instance_transforms.len() as u64
                * std::mem::size_of::<MeshModelMatrix>() as u64;

            let vertex_buffer = self
                .device_context
                .create_buffer(&RafxBufferDef {
                    size: vertex_buffer_size,
                    memory_usage: RafxMemoryUsage::CpuToGpu,
                    resource_type: RafxResourceType::VERTEX_BUFFER,
                    ..Default::default()
                })
                .unwrap();

            // TODO(dvd): Get rid of this copy.
            let mut data = Vec::with_capacity(self.render_object_instance_transforms.len());
            for ii in 0..data.capacity() {
                data.push(self.render_object_instance_transforms.get(ii).clone());
            }

            vertex_buffer
                .copy_to_host_visible_buffer(data.as_slice())
                .unwrap();

            Some(dyn_resource_allocator_set.insert_buffer(vertex_buffer))
        } else {
            None
        };
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    fn new_render_object_instance_job_context(&'prepare self) -> Option<DefaultJobContext> {
        Some(DefaultJobContext::new())
    }

    fn new_render_object_instance_per_view_job_context(
        &'prepare self
    ) -> Option<DefaultJobContext> {
        Some(DefaultJobContext::new())
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = MeshBasicRenderFeatureTypes;
    type SubmitPacketDataT = MeshBasicRenderFeatureTypes;
}
