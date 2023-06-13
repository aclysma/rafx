use fnv::FnvHashMap;
use rafx::render_feature_prepare_job_predule::*;
use std::ops::Mul;

use super::*;
use crate::phases::{
    DepthPrepassRenderPhase, OpaqueRenderPhase, ShadowMapRenderPhase, TransparentRenderPhase,
    WireframeRenderPhase,
};
use rafx::base::resource_map::{ReadBorrow, WriteBorrow};
use rafx::framework::{
    BufferResource, DescriptorSetAllocatorRef, DescriptorSetArc, DescriptorSetBindings,
    DynResourceAllocatorSet, MaterialPassResource, ResourceArc,
};

use crate::shaders::mesh_adv::{
    mesh_adv_textured_frag, mesh_adv_wireframe_vert, mesh_culling_comp,
};
use glam::Mat4;
use rafx::api::{RafxBufferDef, RafxDrawIndexedIndirectCommand, RafxMemoryUsage, RafxResourceType};

use crate::assets::mesh_adv::material_db::MaterialDB;
use crate::assets::mesh_adv::{MeshAdvAssetPart, MeshAdvBlendMethod, MeshAdvShaderPassIndices};
use crate::features::mesh_adv::gpu_occlusion_cull::{
    MeshAdvGpuOcclusionCullRenderResource, OcclusionJob,
};
use crate::features::mesh_adv::light_binning::MeshAdvLightBinRenderResource;
use crate::shaders::depth_velocity::depth_velocity_vert;
use crate::shaders::mesh_adv::lights_bin_comp;
use crate::shaders::mesh_adv::mesh_adv_textured_frag::LightInListStd430;
use crate::shaders::mesh_adv::shadow_atlas_depth_vert;
use mesh_adv_textured_frag::PerViewDataUniform as MeshPerViewFragmentShaderParam;
use rafx::assets::MaterialAsset;
use rafx::renderer::MainViewRenderResource;

const PER_VIEW_DESCRIPTOR_SET_INDEX: u32 =
    mesh_adv_textured_frag::PER_VIEW_DATA_DESCRIPTOR_SET_INDEX as u32;

pub struct MeshAdvPrepareJob<'prepare> {
    #[allow(dead_code)]
    requires_textured_descriptor_sets: bool,
    #[allow(dead_code)]
    requires_untextured_descriptor_sets: bool,
    default_pbr_material: MaterialAsset,
    default_pbr_material_pass_indices: MeshAdvShaderPassIndices,
    depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    shadow_map_atlas_depth_material_pass: Option<ResourceArc<MaterialPassResource>>,
    shadow_map_data: ReadBorrow<'prepare, MeshAdvShadowMapResource>,
    light_bin_resource: WriteBorrow<'prepare, MeshAdvLightBinRenderResource>,
    main_view_resource: ReadBorrow<'prepare, MainViewRenderResource>,
    pipeline_state: ReadBorrow<'prepare, MeshAdvRenderPipelineState>,
    material_db: ReadBorrow<'prepare, MaterialDB>,
    render_object_instance_transforms: Arc<AtomicOnceCellStack<MeshModelMatrix>>,
    render_object_instance_transforms_with_history:
        Arc<AtomicOnceCellStack<MeshModelMatrixWithHistory>>,
    render_object_instance_bounding_spheres:
        Arc<AtomicOnceCellStack<mesh_culling_comp::BoundingSphereBuffer>>,
    #[allow(dead_code)]
    render_objects: MeshAdvRenderObjectSet,
    batched_pass_lookup: AtomicOnceCell<FnvHashMap<MeshAdvBatchedPassKey, usize>>,
    batched_passes: AtomicOnceCell<Vec<MeshAdvBatchedPassInfo>>,
}

impl<'prepare> MeshAdvPrepareJob<'prepare> {
    pub fn new(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<MeshAdvFramePacket>,
        submit_packet: Box<MeshSubmitPacket>,
        render_objects: MeshAdvRenderObjectSet,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        let mut requires_textured_descriptor_sets = false;
        let mut requires_untextured_descriptor_sets = false;

        for view in frame_packet.view_packets() {
            if view
                .view()
                .feature_flag_is_relevant::<MeshAdvUntexturedRenderFeatureFlag>()
            {
                requires_untextured_descriptor_sets = true;
            } else {
                requires_textured_descriptor_sets = true;
            }
        }

        let per_frame_data = frame_packet.per_frame_data().get();
        Arc::new(PrepareJob::new(
            Self {
                render_object_instance_transforms: {
                    // TODO: Ideally this would use an allocator from the `prepare_context`.
                    Arc::new(AtomicOnceCellStack::with_capacity(
                        frame_packet.render_object_instances().len(),
                    ))
                },
                render_object_instance_transforms_with_history: {
                    // TODO: Ideally this would use an allocator from the `prepare_context`.
                    Arc::new(AtomicOnceCellStack::with_capacity(
                        frame_packet.render_object_instances().len(),
                    ))
                },
                render_object_instance_bounding_spheres: {
                    // TODO: Ideally this would use an allocator from the `prepare_context`.
                    Arc::new(AtomicOnceCellStack::with_capacity(
                        frame_packet.render_object_instances().len(),
                    ))
                },
                default_pbr_material: per_frame_data.default_pbr_material.clone(),
                default_pbr_material_pass_indices: per_frame_data
                    .default_pbr_material_pass_indices
                    .clone(),
                depth_material_pass: { per_frame_data.depth_material_pass.clone() },
                shadow_map_atlas_depth_material_pass: {
                    per_frame_data.shadow_map_atlas_depth_material_pass.clone()
                },
                shadow_map_data: {
                    prepare_context
                        .render_resources
                        .fetch::<MeshAdvShadowMapResource>()
                },
                light_bin_resource: {
                    prepare_context
                        .render_resources
                        .fetch_mut::<MeshAdvLightBinRenderResource>()
                },
                main_view_resource: {
                    prepare_context
                        .render_resources
                        .fetch::<MainViewRenderResource>()
                },
                pipeline_state: {
                    prepare_context
                        .render_resources
                        .fetch::<MeshAdvRenderPipelineState>()
                },
                material_db: { prepare_context.render_resources.fetch::<MaterialDB>() },
                requires_textured_descriptor_sets,
                requires_untextured_descriptor_sets,
                render_objects,
                batched_pass_lookup: AtomicOnceCell::new(),
                batched_passes: AtomicOnceCell::new(),
            },
            prepare_context,
            frame_packet,
            submit_packet,
        ))
    }

    // This is a helper function that, given a batch (i.e. opaque meshes, shadow pass meshes, in a view, etc.)
    // we produce a draw data buffer, per-batch descriptor set, and optionally push a batch submit node
    fn prepare_batch<DrawDataT, CreateDrawDataFnT: Fn(&MeshAdvBatchDrawData) -> DrawDataT>(
        job_context: &PreparePerFrameContext<'prepare, '_, Self>,
        descriptor_set_allocator: &mut DescriptorSetAllocatorRef,
        transform_buffer: &ResourceArc<BufferResource>,
        batch: &MeshAdvBatchedPassInfo,
        batch_index: usize,
        push_submit_node: bool,
        per_batch_descriptor_set_index: usize,
        draw_data_binding: u32,
        transforms_binding: u32,
        dyn_resource_allocator_set: &DynResourceAllocatorSet,
        create_draw_data_fn: CreateDrawDataFnT,
    ) -> (ResourceArc<BufferResource>, DescriptorSetArc) {
        let draw_data_buffer_size =
            16 + batch.draw_data.len() as u64 * std::mem::size_of::<DrawDataT>() as u64;

        let draw_data_buffer = dyn_resource_allocator_set.insert_buffer(
            job_context
                .device_context()
                .create_buffer(&RafxBufferDef {
                    size: draw_data_buffer_size,
                    memory_usage: RafxMemoryUsage::CpuToGpu,
                    //DX12TODO: Does not need to be BUFFER_READ_WRITE for other backends
                    resource_type: RafxResourceType::BUFFER | RafxResourceType::BUFFER_READ_WRITE,
                    always_mapped: true,
                    alignment: std::mem::size_of::<DrawDataT>() as u32,
                    ..Default::default()
                })
                .unwrap(),
        );
        let memory = draw_data_buffer.get_raw().buffer.mapped_memory().unwrap();
        unsafe {
            let header = memory as *mut [u32; 4];
            (*header)[0] = batch.draw_data.len() as u32;
            let dst = std::slice::from_raw_parts_mut::<DrawDataT>(
                memory.add(16) as _,
                batch.draw_data.len(),
            );
            let src = batch.draw_data.get_all_unchecked();
            for i in 0..batch.draw_data.len() {
                dst[i] = create_draw_data_fn(&src[i]);
            }
        }

        let layout = &batch.pass.get_raw().descriptor_set_layouts[per_batch_descriptor_set_index];
        let mut dyn_descriptor_set = descriptor_set_allocator
            .create_dyn_descriptor_set_uninitialized(&layout)
            .unwrap();
        dyn_descriptor_set.set_buffer(draw_data_binding, &draw_data_buffer);
        dyn_descriptor_set.set_buffer(transforms_binding, transform_buffer);
        dyn_descriptor_set.flush(descriptor_set_allocator).unwrap();

        if push_submit_node {
            let view_packet = job_context
                .submit_packet()
                .view_submit_packet(batch.view_frame_index);
            view_packet.push_submit_node_into_render_phase(
                batch.phase,
                MeshAdvDrawCall::Batched(MeshAdvBatchedDrawCall {
                    batch_index: batch_index as u32,
                }),
                0,
                0.0,
            );
        }

        (
            draw_data_buffer,
            dyn_descriptor_set.descriptor_set().clone(),
        )
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for MeshAdvPrepareJob<'prepare> {
    fn begin_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        // Helper function that finds/increments the appropriate entry in a hash map (it counts number
        // of draws we will need to allocate for in a batch)
        fn add_batched_pass_count(
            view_packet: &ViewPacket<MeshAdvRenderFeatureTypes>,
            mesh_part: &MeshAdvAssetPart,
            render_phase_index: RenderPhaseIndex,
            pass: &ResourceArc<MaterialPassResource>,
            batched_pass_counts: &mut FnvHashMap<MeshAdvBatchedPassKey, usize>,
        ) {
            if view_packet
                .view()
                .phase_index_is_relevant(render_phase_index)
            {
                let key = MeshAdvBatchedPassKey {
                    phase: render_phase_index,
                    view_frame_index: view_packet.view_frame_index(),
                    pass: pass.clone(),
                    index_type: mesh_part.index_type,
                };
                *batched_pass_counts.entry(key).or_default() += 1;
            }
        }

        //
        // Determine how large our batches will be for anything that can be submitted as a single large batch
        // (anything that doesn't need to be sorted by depth)
        //
        let mut batched_pass_counts = FnvHashMap::<_, usize>::default();
        for view_packet in context.frame_packet().view_packets() {
            for object_instance in view_packet.render_object_instances() {
                let render_object_instance_id = object_instance.render_object_instance_id as usize;
                let render_object_instance_data = context
                    .frame_packet()
                    .render_object_instances_data()
                    .get(render_object_instance_id)
                    .as_ref()
                    .unwrap();
                for mesh_part in &render_object_instance_data.mesh_asset.inner.mesh_parts {
                    let is_transparent = mesh_part.mesh_material.material_data().blend_method
                        != MeshAdvBlendMethod::Opaque;

                    if !is_transparent {
                        if let Some(depth_material_pass) = &self.depth_material_pass {
                            add_batched_pass_count(
                                view_packet,
                                mesh_part,
                                DepthPrepassRenderPhase::render_phase_index(),
                                depth_material_pass,
                                &mut batched_pass_counts,
                            );
                        }

                        if let Some(shadow_map_atlas_depth_material_pass) =
                            &self.shadow_map_atlas_depth_material_pass
                        {
                            add_batched_pass_count(
                                view_packet,
                                mesh_part,
                                ShadowMapRenderPhase::render_phase_index(),
                                shadow_map_atlas_depth_material_pass,
                                &mut batched_pass_counts,
                            );
                        }
                    }

                    let wireframe_pass = self
                        .default_pbr_material
                        .get_material_pass_by_index(
                            self.default_pbr_material_pass_indices.wireframe as usize,
                        )
                        .clone()
                        .unwrap();

                    if view_packet
                        .view()
                        .feature_flag_is_relevant::<MeshAdvWireframeRenderFeatureFlag>()
                    {
                        add_batched_pass_count(
                            view_packet,
                            mesh_part,
                            WireframeRenderPhase::render_phase_index(),
                            &wireframe_pass,
                            &mut batched_pass_counts,
                        );
                    }

                    let render_phase_index = if is_transparent {
                        TransparentRenderPhase::render_phase_index()
                    } else {
                        OpaqueRenderPhase::render_phase_index()
                    };

                    let pass = mesh_part
                        .get_material_pass_resource(view_packet.view(), render_phase_index)
                        .clone();

                    add_batched_pass_count(
                        view_packet,
                        mesh_part,
                        render_phase_index,
                        &pass,
                        &mut batched_pass_counts,
                    );
                }
            }
        }

        //
        // Allocate draw data buffers
        //
        let mut batched_pass_lookup = FnvHashMap::default();
        let mut batched_passes = Vec::default();
        for (key, count) in batched_pass_counts {
            // log::trace!(
            //     "batch index {} key view={} index_type={:?} pass={:?}, count={}",
            //     batched_passes.len(),
            //     key.view_index,
            //     key.index_type,
            //     key.pass.get_raw().material_pass_key,
            //     count
            // );
            let pass = key.pass.clone();
            let pass_info = MeshAdvBatchedPassInfo {
                phase: key.phase,
                pass,
                draw_data: AtomicOnceCellStack::with_capacity(count),
                view_frame_index: key.view_frame_index,
                index_type: key.index_type,
            };

            batched_pass_lookup.insert(key, batched_passes.len());
            batched_passes.push(pass_info);
        }

        self.batched_pass_lookup.set(batched_pass_lookup);
        self.batched_passes.set(batched_passes);

        let mut per_frame_submit_data = Box::new(MeshAdvPerFrameSubmitData {
            num_shadow_map_2d: 0,
            shadow_map_2d_data: [mesh_adv_textured_frag::ShadowMap2DDataUniform {
                ..Default::default()
            }; MAX_SHADOW_MAPS_2D],
            num_shadow_map_cube: 0,
            shadow_map_cube_data: [mesh_adv_textured_frag::ShadowMapCubeDataUniform {
                ..Default::default()
            }; MAX_SHADOW_MAPS_CUBE],
            shadow_map_image_index_remap: Default::default(),
            model_matrix_buffer: Default::default(),
            model_matrix_with_history_buffer: Default::default(),
            all_materials_descriptor_set: Default::default(),
            batched_pass_lookup: Default::default(),
            batched_passes: Default::default(),
            per_batch_descriptor_sets: Default::default(),
            indirect_buffer: Default::default(),
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
                    MeshAdvShadowMapRenderViewIndices::Single(shadow_view_index) => {
                        let shadow_assignment =
                            shadow_map_data.shadow_map_atlas_element_assignment(*shadow_view_index);
                        let shadow_view =
                            shadow_map_data.shadow_map_render_views_meta(*shadow_view_index);

                        let num_shadow_map_2d = per_frame_submit_data.num_shadow_map_2d;
                        if num_shadow_map_2d >= MAX_SHADOW_MAPS_2D {
                            log::warn!(
                                "More 2D shadow maps than the mesh shader can support {}",
                                MAX_SHADOW_MAPS_2D
                            );
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
                    MeshAdvShadowMapRenderViewIndices::Cube(shadow_views) => {
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
            extracted_data.transform.scale,
            extracted_data.transform.rotation,
            extracted_data.transform.translation,
        );

        let previous_world_transform = extracted_data
            .previous_transform
            .as_ref()
            .map(|x| Mat4::from_scale_rotation_translation(x.scale, x.rotation, x.translation))
            .unwrap_or(world_transform);

        let model = world_transform.to_cols_array_2d();
        let previous_model = previous_world_transform.to_cols_array_2d();
        let model_matrix_offset = self
            .render_object_instance_transforms
            .push(MeshModelMatrix {
                model_matrix: model,
            });
        let model_matrix_with_history_offset = self
            .render_object_instance_transforms_with_history
            .push(MeshModelMatrixWithHistory {
                current_model_matrix: model,
                previous_model_matrix: previous_model,
            });

        let bounding_sphere = extracted_data
            .bounding_sphere
            .map(|x| {
                //TODO: Do this in the compute shader if possible. For now do it on CPU so I know it
                // matches frustum culling
                let t = extracted_data.transform;
                let position = t.translation + t.rotation.mul(x.position) * t.scale;
                let radius = x.radius * t.scale.abs().max_element();
                mesh_culling_comp::BoundingSphereBuffer {
                    position: position.into(),
                    radius: radius,
                }
            })
            .unwrap_or_else(|| mesh_culling_comp::BoundingSphereBuffer {
                position: [0.0, 0.0, 0.0],
                radius: -1.0,
            });

        let bounding_spheres_offset = self
            .render_object_instance_bounding_spheres
            .push(bounding_sphere);

        debug_assert_eq!(model_matrix_offset, model_matrix_with_history_offset);
        debug_assert_eq!(model_matrix_offset, bounding_spheres_offset);

        context.set_render_object_instance_submit_data(MeshAdvRenderObjectInstanceSubmitData {
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
            let distance =
                (view.eye_position() - extracted_data.transform.translation).length_squared();
            let render_object_instance_id = context.render_object_instance_id();

            let model_matrix_offset = context
                .render_object_instance_submit_data()
                .model_matrix_offset;

            #[derive(Debug)]
            struct PushDrawDataResult {
                batch_index: u32,
                draw_data_index: u32,
            }

            // helper function that finds the correct batch and pushes draw data into it
            fn push_draw_data(
                view_frame_index: ViewFrameIndex,
                render_phase_index: RenderPhaseIndex,
                pass: ResourceArc<MaterialPassResource>,
                mesh_part: &MeshAdvAssetPart,
                batched_passes: &AtomicOnceCell<Vec<MeshAdvBatchedPassInfo>>,
                batched_pass_lookup: &AtomicOnceCell<FnvHashMap<MeshAdvBatchedPassKey, usize>>,
                model_matrix_offset: usize,
                mesh_part_material_index: u32,
                use_full_vertices: bool,
            ) -> PushDrawDataResult {
                let batch_key = MeshAdvBatchedPassKey {
                    phase: render_phase_index,
                    view_frame_index,
                    pass,
                    index_type: mesh_part.index_type,
                };
                //println!("pushing to key view={} index_type={:?} pass={:?}", batch_key.view_index, batch_key.index_type, batch_key.pass.get_raw().material_pass_key);
                let batch_index = *batched_pass_lookup.get().get(&batch_key).unwrap();
                //println!("batch index {}", batch_index);

                let batched_pass = &batched_passes.get()[batch_index];
                let draw_data_index = batched_pass.draw_data.len() as u32;

                let (vertex_size, vertex_buffer_offset_in_bytes) = if use_full_vertices {
                    (
                        std::mem::size_of::<MeshVertexFull>() as u32,
                        mesh_part.vertex_full_buffer_offset_in_bytes,
                    )
                } else {
                    (
                        std::mem::size_of::<MeshVertexPosition>() as u32,
                        mesh_part.vertex_position_buffer_offset_in_bytes,
                    )
                };

                assert!(vertex_buffer_offset_in_bytes % vertex_size as u32 == 0);
                let vertex_offset = vertex_buffer_offset_in_bytes / vertex_size;

                let index_size_in_bytes = mesh_part.index_type.size_in_bytes() as u32;
                assert!(mesh_part.index_buffer_size_in_bytes % index_size_in_bytes == 0);
                assert!(mesh_part.index_buffer_offset_in_bytes % index_size_in_bytes == 0);

                batched_pass.draw_data.push(MeshAdvBatchDrawData {
                    material_index: mesh_part_material_index,
                    index_count: mesh_part.index_buffer_size_in_bytes / index_size_in_bytes,
                    index_offset: mesh_part.index_buffer_offset_in_bytes / index_size_in_bytes,
                    transform_index: model_matrix_offset as u32,
                    vertex_offset,
                });

                PushDrawDataResult {
                    batch_index: batch_index as u32,
                    draw_data_index,
                }
            }

            //
            // Iterate all mesh parts and push draw calls into batches. Additionally push submit nodes
            // for transparent meshes as we need to sort these by depth and draw them individually
            //
            for (mesh_part_index, mesh_part) in extracted_data
                .mesh_asset
                .inner
                .mesh_parts
                .iter()
                .enumerate()
            {
                let is_transparent = mesh_part.mesh_material.material_data().blend_method
                    != MeshAdvBlendMethod::Opaque;
                let mesh_part_material_index =
                    mesh_part.mesh_material.inner.material.material_data_index();

                if !is_transparent {
                    if view.phase_is_relevant::<DepthPrepassRenderPhase>()
                        && self.depth_material_pass.is_some()
                    {
                        let pass = self.depth_material_pass.as_ref().unwrap().clone();
                        push_draw_data(
                            context.view_frame_index(),
                            DepthPrepassRenderPhase::render_phase_index(),
                            pass,
                            mesh_part,
                            &self.batched_passes,
                            &self.batched_pass_lookup,
                            model_matrix_offset,
                            mesh_part_material_index,
                            false,
                        );
                    }

                    if view.phase_is_relevant::<ShadowMapRenderPhase>()
                        && self.shadow_map_atlas_depth_material_pass.is_some()
                    {
                        let pass = self
                            .shadow_map_atlas_depth_material_pass
                            .as_ref()
                            .unwrap()
                            .clone();
                        push_draw_data(
                            context.view_frame_index(),
                            ShadowMapRenderPhase::render_phase_index(),
                            pass,
                            mesh_part,
                            &self.batched_passes,
                            &self.batched_pass_lookup,
                            model_matrix_offset,
                            mesh_part_material_index,
                            false,
                        );
                    }
                }

                let phase_index = if !is_transparent {
                    OpaqueRenderPhase::render_phase_index()
                } else {
                    TransparentRenderPhase::render_phase_index()
                };

                if view.phase_index_is_relevant(phase_index) {
                    let material_pass_resource = mesh_part
                        .get_material_pass_resource(view, phase_index)
                        .clone();

                    let push_draw_data_result = push_draw_data(
                        context.view_frame_index(),
                        phase_index,
                        material_pass_resource.clone(),
                        mesh_part,
                        &self.batched_passes,
                        &self.batched_pass_lookup,
                        model_matrix_offset,
                        mesh_part_material_index,
                        true,
                    );

                    //
                    // We only write per-element submit nodes if we need to do depth sorting (i.e. transparent meshes)
                    //
                    if is_transparent {
                        context.push_submit_node_into_render_phase(
                            phase_index,
                            MeshAdvDrawCall::Unbatched(MeshAdvUnbatchedDrawCall {
                                render_object_instance_id,
                                material_pass_resource,
                                mesh_part_index,
                                model_matrix_index: model_matrix_offset,
                                material_index: Some(
                                    mesh_part.mesh_material.inner.material.material_data_index(),
                                ),
                                draw_data_index: push_draw_data_result.draw_data_index,
                                index_type: mesh_part.index_type,
                                batch_index: push_draw_data_result.batch_index,
                            }),
                            push_draw_data_result.batch_index,
                            distance,
                        );
                    }
                }

                if view.phase_is_relevant::<WireframeRenderPhase>()
                    && view.feature_flag_is_relevant::<MeshAdvWireframeRenderFeatureFlag>()
                {
                    let material_pass_resource = self
                        .default_pbr_material
                        .get_material_pass_by_index(
                            self.default_pbr_material_pass_indices.wireframe as usize,
                        )
                        .clone()
                        .unwrap();

                    push_draw_data(
                        context.view_frame_index(),
                        WireframeRenderPhase::render_phase_index(),
                        material_pass_resource,
                        mesh_part,
                        &self.batched_passes,
                        &self.batched_pass_lookup,
                        model_matrix_offset,
                        mesh_part_material_index,
                        false,
                    );
                }
            }
        }
    }

    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let mut descriptor_set_allocator =
            context.resource_context().create_descriptor_set_allocator();
        let shadow_map_data = &self.shadow_map_data;

        let per_view_data = context.per_view_data();
        let per_frame_submit_data = context.per_frame_submit_data();

        let view = context.view();
        let is_lit = !view.feature_flag_is_relevant::<MeshAdvUnlitRenderFeatureFlag>();
        let has_shadows = !view.feature_flag_is_relevant::<MeshAdvNoShadowsRenderFeatureFlag>();

        let opaque_descriptor_set = if view.phase_is_relevant::<OpaqueRenderPhase>()
            || view.phase_is_relevant::<TransparentRenderPhase>()
        {
            let mut all_lights_buffer_data = mesh_adv_textured_frag::AllLightsBuffer {
                light_count: 0,
                _padding0: Default::default(),
                data: [LightInListStd430 {
                    position_ws: Default::default(),
                    range: Default::default(),
                    position_vs: Default::default(),
                    intensity: Default::default(),
                    color: Default::default(),
                    spotlight_direction_ws: Default::default(),
                    spotlight_half_angle: Default::default(),
                    spotlight_direction_vs: Default::default(),
                    shadow_map: Default::default(),
                }; 512],
            };

            let per_view_frag_data = {
                let mut per_view_frag_data = MeshPerViewFragmentShaderParam::default();

                per_view_frag_data.view = view.view_matrix().to_cols_array_2d();
                per_view_frag_data.view_proj = view.view_proj().to_cols_array_2d();
                per_view_frag_data.ndf_filter_amount = per_view_data.ndf_filter_amount;
                per_view_frag_data.ambient_light = if is_lit {
                    per_view_data.ambient_light.extend(1.0).into()
                } else {
                    glam::Vec4::ONE.into()
                };
                per_view_frag_data.use_clustered_lighting = if per_view_data.use_clustered_lighting
                {
                    1
                } else {
                    0
                };
                per_view_frag_data.viewport_width = view.extents_width();
                per_view_frag_data.viewport_height = view.extents_height();
                per_view_frag_data.jitter_amount = self.pipeline_state.jitter_amount.into();
                per_view_frag_data.mip_bias = self.pipeline_state.forward_pass_mip_bias;

                let mut light_bounds_data = lights_bin_comp::LightsInputListBuffer {
                    light_count: 0,
                    _padding0: Default::default(),
                    lights: [lights_bin_comp::LightStd430 {
                        position: [0.0, 0.0, 0.0],
                        radius: 0.0,
                    }; 512],
                };

                for light in &per_view_data.directional_lights {
                    let light_count = per_view_frag_data.directional_light_count as usize;
                    if light_count >= per_view_frag_data.directional_lights.len() {
                        break;
                    }

                    let shadow_map_index = shadow_map_data
                        .shadow_map_lookup_by_light_id
                        .get(&MeshAdvLightId::DirectionalLight(light.object_id))
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

                for light in &per_view_data.point_lights {
                    let position_vs = (view.view_matrix()
                        * light.transform.translation.extend(1.0))
                    .truncate()
                    .into();
                    let range = light.light.range();

                    let shadow_map_index = shadow_map_data
                        .shadow_map_lookup_by_light_id
                        .get(&MeshAdvLightId::PointLight(light.object_id))
                        .map(|x| {
                            per_frame_submit_data.shadow_map_image_index_remap[&x.unwrap_cube_any()]
                        });

                    {
                        light_bounds_data.lights[light_bounds_data.light_count as usize] =
                            lights_bin_comp::LightBuffer {
                                position: position_vs,
                                radius: range,
                            };
                        light_bounds_data.light_count += 1;
                    }

                    {
                        let out = &mut all_lights_buffer_data.data
                            [all_lights_buffer_data.light_count as usize];
                        out.position_ws = light.transform.translation.into();
                        out.position_vs = (view.view_matrix()
                            * light.transform.translation.extend(1.0))
                        .truncate()
                        .into();
                        out.color = light.light.color.into();
                        out.range = light.light.range();
                        out.intensity = light.light.intensity;
                        out.shadow_map = if has_shadows {
                            shadow_map_index.map(|x| x as i32).unwrap_or(-1)
                        } else {
                            -1
                        };

                        all_lights_buffer_data.light_count += 1;
                    }
                }

                for light in &per_view_data.spot_lights {
                    let light_from = light.transform.translation;
                    let light_from_vs = (view.view_matrix() * light_from.extend(1.0)).truncate();

                    let light_to = light.transform.translation + light.light.direction;
                    let light_to_vs = (view.view_matrix() * light_to.extend(1.0)).truncate();

                    let light_direction = (light_to - light_from).normalize();
                    let light_direction_vs = (light_to_vs - light_from_vs).normalize();

                    let range = light.light.range();

                    let shadow_map_index = shadow_map_data
                        .shadow_map_lookup_by_light_id
                        .get(&MeshAdvLightId::SpotLight(light.object_id))
                        .map(|x| {
                            per_frame_submit_data
                                .shadow_map_image_index_remap
                                .get(&x.unwrap_single())
                        })
                        .flatten();

                    {
                        light_bounds_data.lights[light_bounds_data.light_count as usize] =
                            lights_bin_comp::LightBuffer {
                                position: light_from_vs.into(),
                                radius: range,
                            };
                        light_bounds_data.light_count += 1;
                    }

                    {
                        let out = &mut all_lights_buffer_data.data
                            [all_lights_buffer_data.light_count as usize];
                        out.position_ws = light_from.into();
                        out.position_vs = light_from_vs.into();
                        out.spotlight_direction_ws = light_direction.into();
                        out.spotlight_direction_vs = light_direction_vs.into();
                        out.spotlight_half_angle = light.light.spotlight_half_angle;
                        out.color = light.light.color.into();
                        out.range = light.light.range();
                        out.intensity = light.light.intensity;
                        out.shadow_map = if has_shadows {
                            shadow_map_index.map(|x| *x as i32).unwrap_or(-1)
                        } else {
                            -1
                        };

                        all_lights_buffer_data.light_count += 1;
                    }
                }

                per_view_frag_data.shadow_map_2d_data = per_frame_submit_data.shadow_map_2d_data;
                per_view_frag_data.shadow_map_cube_data =
                    per_frame_submit_data.shadow_map_cube_data;

                self.light_bin_resource
                    .update_light_bounds(context.view().frame_index(), &light_bounds_data)
                    .unwrap();

                per_view_frag_data
            };

            let all_lights_buffer = {
                let dyn_resource_allocator_set = context
                    .resource_context()
                    .create_dyn_resource_allocator_set();

                let all_lights_buffer_size =
                    std::mem::size_of::<mesh_adv_textured_frag::AllLightsBuffer>();
                let all_lights_buffer = context
                    .device_context()
                    .create_buffer(&RafxBufferDef {
                        size: all_lights_buffer_size as u64,
                        memory_usage: RafxMemoryUsage::CpuToGpu,
                        //DX12TODO: Does not need to be BUFFER_READ_WRITE for other backends
                        resource_type: RafxResourceType::BUFFER_READ_WRITE,
                        ..Default::default()
                    })
                    .unwrap();

                all_lights_buffer
                    .copy_to_host_visible_buffer(&[all_lights_buffer_data])
                    .unwrap();

                dyn_resource_allocator_set.insert_buffer(all_lights_buffer)
            };

            let shadow_map_atlas = context.per_frame_data().shadow_map_atlas.clone();

            // NOTE(dvd): This assumes that all opaque materials have the same per view descriptor set layout.
            let default_opaque_pass = self
                .default_pbr_material
                .get_material_pass_by_index(self.default_pbr_material_pass_indices.opaque as usize)
                .unwrap();
            let opaque_per_view_descriptor_set_layout = default_opaque_pass
                .get_raw()
                .descriptor_set_layouts[PER_VIEW_DESCRIPTOR_SET_INDEX as usize]
                .clone();

            let mut dyn_descriptor_set = descriptor_set_allocator
                .create_dyn_descriptor_set_uninitialized(&opaque_per_view_descriptor_set_layout)
                .unwrap();
            dyn_descriptor_set.set_buffer_data(
                mesh_adv_textured_frag::PER_VIEW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
                &per_view_frag_data,
            );
            dyn_descriptor_set.set_image(
                mesh_adv_textured_frag::SHADOW_MAP_ATLAS_DESCRIPTOR_BINDING_INDEX as u32,
                &shadow_map_atlas,
            );
            dyn_descriptor_set.set_buffer(
                mesh_adv_textured_frag::LIGHT_BIN_OUTPUT_DESCRIPTOR_BINDING_INDEX as u32,
                self.light_bin_resource
                    .output_gpu_buffer(view.frame_index()),
            );
            dyn_descriptor_set.set_buffer(
                mesh_adv_textured_frag::ALL_LIGHTS_DESCRIPTOR_BINDING_INDEX as u32,
                &all_lights_buffer,
            );
            dyn_descriptor_set
                .flush(&mut descriptor_set_allocator)
                .unwrap();
            Some(dyn_descriptor_set.descriptor_set().clone())
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
            let mut per_view_data = depth_velocity_vert::PerViewDataUniform::default();

            per_view_data.current_view_proj = view.view_proj().to_cols_array_2d();
            per_view_data.current_view_proj_inv = view.view_proj().inverse().to_cols_array_2d();
            per_view_data.viewport_width = view.extents_width();
            per_view_data.viewport_height = view.extents_height();
            per_view_data.jitter_amount = self.pipeline_state.jitter_amount.into();

            if let Some(previous_main_view_info) = &self.main_view_resource.previous_main_view_info
            {
                let previous_view_proj =
                    previous_main_view_info.projection_matrix * previous_main_view_info.view_matrix;
                per_view_data.previous_view_proj = previous_view_proj.to_cols_array_2d();
            } else {
                per_view_data.previous_view_proj = per_view_data.current_view_proj;
            }

            let per_instance_descriptor_set_layout = &self
                .depth_material_pass
                .as_ref()
                .unwrap()
                .get_raw()
                .descriptor_set_layouts[PER_VIEW_DESCRIPTOR_SET_INDEX as usize];

            descriptor_set_allocator
                .create_descriptor_set(
                    per_instance_descriptor_set_layout,
                    depth_velocity_vert::DescriptorSet0Args {
                        per_view_data: &per_view_data,
                    },
                )
                .ok()
        } else {
            None
        };

        //
        // If we are rendering a wireframe, make a scriptor set with uniform data for this view
        //
        let wireframe_desriptor_set = if view.phase_is_relevant::<WireframeRenderPhase>() {
            let mut per_view_data = mesh_adv_wireframe_vert::PerViewDataUniform::default();

            per_view_data.view = view.view_matrix().to_cols_array_2d();
            per_view_data.view_proj = view.view_proj().to_cols_array_2d();

            let per_instance_descriptor_set_layout = &self
                .default_pbr_material
                .get_material_pass_by_index(
                    self.default_pbr_material_pass_indices.wireframe as usize,
                )
                .as_ref()
                .unwrap()
                .get_raw()
                .descriptor_set_layouts[PER_VIEW_DESCRIPTOR_SET_INDEX as usize];

            descriptor_set_allocator
                .create_descriptor_set(
                    per_instance_descriptor_set_layout,
                    mesh_adv_wireframe_vert::DescriptorSet0Args {
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
            .set(MeshAdvPerViewSubmitData {
                opaque_descriptor_set,
                depth_descriptor_set,
                shadow_map_atlas_depth_descriptor_set,
                wireframe_desriptor_set,
            });
    }

    fn end_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        let dyn_resource_allocator_set = context
            .resource_context()
            .create_dyn_resource_allocator_set();
        let mut descriptor_set_allocator =
            context.resource_context().create_descriptor_set_allocator();

        // Helper function that allocates a buffer to hold transforms for all draws in a batch and
        // copies data into it
        fn create_buffer_from_atomic_once_stack<T: Copy + 'static>(
            dyn_resource_allocator_set: &DynResourceAllocatorSet,
            transforms: &Arc<AtomicOnceCellStack<T>>,
            memory_usage: RafxMemoryUsage,
            resource_type: RafxResourceType,
        ) -> Option<ResourceArc<BufferResource>> {
            if transforms.len() > 0 {
                let buffer_size = transforms.len() as u64 * std::mem::size_of::<T>() as u64;

                let buffer = dyn_resource_allocator_set
                    .device_context
                    .create_buffer(&RafxBufferDef {
                        size: buffer_size,
                        memory_usage,
                        resource_type,
                        ..Default::default()
                    })
                    .unwrap();

                let data = unsafe { transforms.get_all_unchecked() };

                buffer.copy_to_host_visible_buffer(data).unwrap();

                Some(dyn_resource_allocator_set.insert_buffer(buffer))
            } else {
                None
            }
        }

        //
        // Create buffers for transforms
        //
        let all_transforms = create_buffer_from_atomic_once_stack(
            &dyn_resource_allocator_set,
            &self.render_object_instance_transforms,
            RafxMemoryUsage::CpuToGpu,
            //DX12TODO: Does not need to be BUFFER_READ_WRITE for other backends
            RafxResourceType::BUFFER_READ_WRITE,
        );

        let mut model_matrix_buffer = context
            .per_frame_submit_data()
            .model_matrix_buffer
            .borrow_mut();

        *model_matrix_buffer = all_transforms.clone();

        let mut model_matrix_with_history_buffer = context
            .per_frame_submit_data()
            .model_matrix_with_history_buffer
            .borrow_mut();

        *model_matrix_with_history_buffer = create_buffer_from_atomic_once_stack(
            &dyn_resource_allocator_set,
            &self.render_object_instance_transforms_with_history,
            RafxMemoryUsage::CpuToGpu,
            //DX12TODO: Does not need to be BUFFER_READ_WRITE for other backends
            RafxResourceType::BUFFER_READ_WRITE,
        );

        let bounding_spheres_buffer = create_buffer_from_atomic_once_stack(
            &dyn_resource_allocator_set,
            &self.render_object_instance_bounding_spheres,
            RafxMemoryUsage::CpuToGpu,
            //DX12TODO: Does not need to be BUFFER_READ_WRITE for other backends
            RafxResourceType::BUFFER_READ_WRITE,
        );

        //
        // Update the material DB and get the descriptor set with all data/textures
        //
        let mut all_materials_descriptor_set = context
            .per_frame_submit_data()
            .all_materials_descriptor_set
            .borrow_mut();

        let pbr_material_descriptor_layout = &self
            .default_pbr_material
            .get_material_pass_by_index(self.default_pbr_material_pass_indices.opaque as usize)
            .unwrap()
            .get_raw()
            .descriptor_set_layouts
            [mesh_adv_textured_frag::ALL_MATERIALS_DESCRIPTOR_SET_INDEX as usize];

        let invalid_image_color = context
            .frame_packet()
            .per_frame_data()
            .get()
            .invalid_image_color
            .clone();

        *all_materials_descriptor_set = Some(
            self.material_db
                .update_gpu_resources(
                    context.resource_context(),
                    pbr_material_descriptor_layout,
                    &invalid_image_color,
                )
                .unwrap(),
        );

        //NOTE: We make indirect commands even for non-batched render nodes so that we can do GPU
        // culling with them
        let mut all_indirect_commands_count = 0;
        for pass in self.batched_passes.get() {
            all_indirect_commands_count += pass.draw_data.len()
        }

        let command_size =
            rafx::api::extra::indirect::indexed_indirect_command_size(context.device_context());

        let indirect_buffer_size_in_bytes =
            all_indirect_commands_count as u64 * command_size as u64;
        let indirect_buffer = dyn_resource_allocator_set.insert_buffer(
            dyn_resource_allocator_set
                .device_context
                .create_buffer(&RafxBufferDef {
                    size: indirect_buffer_size_in_bytes,
                    memory_usage: RafxMemoryUsage::CpuToGpu,
                    //DX12TODO: Does not need to be BUFFER_READ_WRITE for other backends
                    resource_type: RafxResourceType::BUFFER
                        | RafxResourceType::BUFFER_READ_WRITE
                        | RafxResourceType::INDIRECT_BUFFER,
                    always_mapped: true,
                    alignment: command_size as u32,
                    ..Default::default()
                })
                .unwrap(),
        );
        indirect_buffer
            .get_raw()
            .buffer
            .set_debug_name("MeshAdv indirect buffer");
        let indirect_buffer_ref = indirect_buffer.get_raw().buffer;
        let indirect_buffer_encoder =
            rafx::api::RafxIndexedIndirectCommandEncoder::new(&*indirect_buffer_ref);

        let mut indirect_buffer_next_command_index = 0;

        //
        // Do final processing for each batch (produces a draw data buffer, per-batch descriptor
        // set, and optionally push a batch submit node. (Transforms should *not* push submit nodes
        // because we already pushed per-draw submit nodes, as we need to support them by depth)
        //
        let mut draw_data_buffers = Vec::with_capacity(self.batched_passes.get().len());
        let mut per_batch_descriptor_sets = Vec::with_capacity(self.batched_passes.get().len());
        for (batch_index, batch) in self.batched_passes.get().iter().enumerate() {
            let per_batch_descriptor_set =
                if batch.phase == ShadowMapRenderPhase::render_phase_index() {
                    Some(Self::prepare_batch(
                        context,
                        &mut descriptor_set_allocator,
                        model_matrix_buffer.as_ref().unwrap(),
                        batch,
                        batch_index,
                        true,
                        shadow_atlas_depth_vert::ALL_DRAW_DATA_DESCRIPTOR_SET_INDEX,
                        shadow_atlas_depth_vert::ALL_DRAW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
                        shadow_atlas_depth_vert::ALL_TRANSFORMS_DESCRIPTOR_BINDING_INDEX as u32,
                        &dyn_resource_allocator_set,
                        |src| shadow_atlas_depth_vert::DrawDataBuffer {
                            transform_index: src.transform_index,
                            material_index: src.material_index,
                        },
                    ))
                } else if batch.phase == DepthPrepassRenderPhase::render_phase_index() {
                    Some(Self::prepare_batch(
                        context,
                        &mut descriptor_set_allocator,
                        model_matrix_with_history_buffer.as_ref().unwrap(),
                        batch,
                        batch_index,
                        true,
                        depth_velocity_vert::ALL_DRAW_DATA_DESCRIPTOR_SET_INDEX,
                        depth_velocity_vert::ALL_DRAW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
                        depth_velocity_vert::ALL_TRANSFORMS_DESCRIPTOR_BINDING_INDEX as u32,
                        &dyn_resource_allocator_set,
                        |src| depth_velocity_vert::DrawDataBuffer {
                            transform_index: src.transform_index,
                            material_index: src.material_index,
                        },
                    ))
                } else if batch.phase == OpaqueRenderPhase::render_phase_index()
                    || batch.phase == TransparentRenderPhase::render_phase_index()
                {
                    let push_submit_node = batch.phase == OpaqueRenderPhase::render_phase_index();
                    Some(Self::prepare_batch(
                        context,
                        &mut descriptor_set_allocator,
                        model_matrix_buffer.as_ref().unwrap(),
                        batch,
                        batch_index,
                        push_submit_node,
                        mesh_adv_textured_frag::ALL_DRAW_DATA_DESCRIPTOR_SET_INDEX,
                        mesh_adv_textured_frag::ALL_DRAW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
                        mesh_adv_textured_frag::ALL_TRANSFORMS_DESCRIPTOR_BINDING_INDEX as u32,
                        &dyn_resource_allocator_set,
                        |src| mesh_adv_textured_frag::DrawDataBuffer {
                            transform_index: src.transform_index,
                            material_index: src.material_index,
                        },
                    ))
                } else if batch.phase == WireframeRenderPhase::render_phase_index() {
                    Some(Self::prepare_batch(
                        context,
                        &mut descriptor_set_allocator,
                        model_matrix_buffer.as_ref().unwrap(),
                        batch,
                        batch_index,
                        true,
                        mesh_adv_wireframe_vert::ALL_DRAW_DATA_DESCRIPTOR_SET_INDEX,
                        mesh_adv_wireframe_vert::ALL_DRAW_DATA_DESCRIPTOR_BINDING_INDEX as u32,
                        mesh_adv_wireframe_vert::ALL_TRANSFORMS_DESCRIPTOR_BINDING_INDEX as u32,
                        &dyn_resource_allocator_set,
                        |src| mesh_adv_wireframe_vert::DrawDataBuffer {
                            transform_index: src.transform_index,
                            material_index: src.material_index,
                        },
                    ))
                } else {
                    None
                };

            if let Some((draw_data_buffer, per_batch_descriptor_set)) = per_batch_descriptor_set {
                draw_data_buffers.push(Some(draw_data_buffer));
                per_batch_descriptor_sets.push(Some(per_batch_descriptor_set));
            } else {
                draw_data_buffers.push(None);
                per_batch_descriptor_sets.push(None);
            }
        }

        descriptor_set_allocator.flush_changes().unwrap();

        let mut prepared_batch_data = Vec::default();
        for pass_info in self.batched_passes.get() {
            let indirect_buffer_first_command_index = indirect_buffer_next_command_index as u32;
            let draw_data_count = pass_info.draw_data.len() as u32;

            let is_batched = pass_info.phase != TransparentRenderPhase::render_phase_index();
            let draw_data = if !is_batched {
                let mut draw_data = Vec::with_capacity(draw_data_count as usize);
                unsafe {
                    for dd in pass_info.draw_data.get_all_unchecked() {
                        draw_data.push(dd.clone());
                    }
                }
                Some(draw_data)
            } else {
                None
            };

            for (i, src) in pass_info.draw_data.iter().enumerate() {
                indirect_buffer_encoder.set_command(
                    indirect_buffer_next_command_index,
                    RafxDrawIndexedIndirectCommand {
                        index_count: src.index_count,
                        instance_count: 1,
                        first_index: src.index_offset,
                        vertex_offset: src.vertex_offset as i32,
                        first_instance: i as u32,
                    },
                );
                indirect_buffer_next_command_index += 1;
            }

            assert_eq!(
                indirect_buffer_next_command_index as u32 - indirect_buffer_first_command_index,
                draw_data_count
            );
            prepared_batch_data.push(MeshAdvBatchedPreparedPassInfo {
                pass: pass_info.pass.clone(),
                phase: pass_info.phase,
                index_type: pass_info.index_type,
                indirect_buffer_first_command_index,
                indirect_buffer_command_count: draw_data_count,
                draw_data,
            })
        }

        assert_eq!(
            indirect_buffer_next_command_index,
            all_indirect_commands_count
        );

        let mut occlusion_cull_resource = context
            .render_resources()
            .fetch_mut::<MeshAdvGpuOcclusionCullRenderResource>();
        occlusion_cull_resource.data.clear();

        for (batch_index, prepared_pass_info) in prepared_batch_data.iter().enumerate() {
            if prepared_pass_info.phase == OpaqueRenderPhase::render_phase_index()
                || prepared_pass_info.phase == TransparentRenderPhase::render_phase_index()
            {
                let pass_info = &self.batched_passes.get()[batch_index];

                let draw_data_count = pass_info.draw_data.len() as u32;
                if draw_data_count == 0 {
                    continue;
                }

                let view = context
                    .frame_packet()
                    .view_packet(pass_info.view_frame_index)
                    .view();

                if let Some(draw_data_buffer) = &draw_data_buffers[batch_index] {
                    // Set indirect_buffer, all_transforms, and bounding_spheres_buffer volume buffers for later usage?
                    occlusion_cull_resource.data.push(OcclusionJob {
                        draw_data_count,
                        render_view: view.clone(),
                        indirect_first_command_index: prepared_pass_info
                            .indirect_buffer_first_command_index,
                        draw_data: draw_data_buffer.clone(),
                        transforms: all_transforms.clone().unwrap(),
                        bounding_spheres: bounding_spheres_buffer.clone().unwrap(),
                        indirect_commands: indirect_buffer.clone(),
                    });
                }
            }
        }

        context
            .per_frame_submit_data()
            .batched_pass_lookup
            .set(self.batched_pass_lookup.get().clone());
        context
            .per_frame_submit_data()
            .batched_passes
            .set(prepared_batch_data);
        context
            .per_frame_submit_data()
            .per_batch_descriptor_sets
            .set(per_batch_descriptor_sets);
        context
            .per_frame_submit_data()
            .indirect_buffer
            .set(indirect_buffer);
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

    type FramePacketDataT = MeshAdvRenderFeatureTypes;
    type SubmitPacketDataT = MeshAdvRenderFeatureTypes;
}
