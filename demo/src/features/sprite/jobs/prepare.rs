use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::{OpaqueRenderPhase, TransparentRenderPhase};
use fnv::FnvHashMap;
use rafx::api::{RafxBufferDef, RafxDeviceContext, RafxMemoryUsage, RafxResourceType};
use rafx::base::DecimalF32;
use rafx::framework::{ImageViewResource, ResourceArc, ResourceContext};
use std::sync::atomic::{AtomicU32, Ordering};

/// Used as static data to represent a quad
#[derive(Clone, Debug, Copy)]
struct QuadVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
}

/// Static data the represents a "unit" quad
const QUAD_VERTEX_LIST: [QuadVertex; 4] = [
    // Top Right
    QuadVertex {
        pos: [0.5, 0.5, 0.0],
        tex_coord: [1.0, 0.0],
    },
    // Top Left
    QuadVertex {
        pos: [-0.5, 0.5, 0.0],
        tex_coord: [0.0, 0.0],
    },
    // Bottom Right
    QuadVertex {
        pos: [0.5, -0.5, 0.0],
        tex_coord: [1.0, 1.0],
    },
    // Bottom Left
    QuadVertex {
        pos: [-0.5, -0.5, 0.0],
        tex_coord: [0.0, 1.0],
    },
];

/// Draw order of QUAD_VERTEX_LIST
const QUAD_INDEX_LIST: [u16; 6] = [0, 1, 2, 2, 1, 3];

pub struct SpritePrepareJob {
    resource_context: ResourceContext,
    device_context: RafxDeviceContext,
    render_objects: SpriteRenderObjectSet,
}

impl SpritePrepareJob {
    pub fn new<'prepare>(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<SpriteFramePacket>,
        submit_packet: Box<SpriteSubmitPacket>,
        render_objects: SpriteRenderObjectSet,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self {
                resource_context: prepare_context.resource_context.clone(),
                device_context: prepare_context.device_context.clone(),
                render_objects,
            },
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for SpritePrepareJob {
    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        if per_frame_data.sprite_material_pass.is_none() {
            return;
        }

        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();
        let dyn_resource_allocator_set = self.resource_context.create_dyn_resource_allocator_set();

        let sprite_material_pass = per_frame_data.sprite_material_pass.as_ref().unwrap();
        let per_view_descriptor_set_layout = &sprite_material_pass.get_raw().descriptor_set_layouts
            [shaders::sprite_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX];

        let view = context.view();
        let view_packet = context.view_packet();
        let view_submit_packet = context.view_submit_packet();

        let per_view_descriptor_set = Some(
            descriptor_set_allocator
                .create_descriptor_set_with_writer(
                    per_view_descriptor_set_layout,
                    shaders::sprite_vert::DescriptorSet0Args {
                        uniform_buffer: &shaders::sprite_vert::ArgsUniform {
                            mvp: view.view_proj().to_cols_array_2d(),
                        },
                    },
                )
                .unwrap(),
        );

        let mut per_view_submit_data = SpritePerViewSubmitData::default();
        per_view_submit_data.descriptor_set_arc = per_view_descriptor_set.clone();

        //
        // Create descriptor sets per distinct image/material. Also assign a batch index to each
        // sprite. We will use it later to combine view nodes with the same batch index into single
        // draw calls
        //

        // Temporary lookup for material index. This allows us to create exactly one per texture we
        // render

        let mut material_lookup = FnvHashMap::<ResourceArc<ImageViewResource>, u32>::default();

        // List of all per material descriptor sets, indexed by material index
        let mut per_material_descriptor_sets = Vec::default();

        // Used to batch sprites by depth and material (depth is None unless the sprite is transparent)
        #[derive(PartialEq, Eq, Hash, Debug)]
        struct BatchKey {
            // f32 is not friendly to use in a map. This wrapper makes it hash bit-wise, which is
            // fine given that inconsistent hashes with the "same" f32 value just mean sprites
            // won't get batched. In practice this is likely to be rare and will not result in
            // "wrong" behavior. Just less effective batching.
            depth: Option<DecimalF32>,
            material_index: u32,
        }

        // Lookup for finding the batch index for by key
        let mut batch_key_lookup = FnvHashMap::<BatchKey, u32>::default();
        let mut batch_count = 0;

        struct PerNodeData {
            batch_index: u32,
            material_index: u32,
        }

        // Batch index for every sprite frame node

        let mut batch_indices =
            Vec::<Option<PerNodeData>>::with_capacity(view_packet.render_object_instances().len());

        {
            profiling::scope!("batch assignment");

            for sprite in view_packet.render_object_instances().iter() {
                if let Some(sprite) = context
                    .render_object_instances_data()
                    .get(sprite.render_object_instance_id as usize)
                    .as_ref()
                {
                    //
                    // First, get or create the descriptor set for the material
                    //
                    //TODO: Cache and reuse where image/material is the same
                    let material_index = *material_lookup
                        .entry(sprite.image_view.clone())
                        .or_insert_with(|| {
                            profiling::scope!("allocate descriptor set");

                            let descriptor_set = descriptor_set_allocator
                                .create_descriptor_set_with_writer(
                                    &sprite_material_pass.get_raw().descriptor_set_layouts
                                        [shaders::sprite_frag::TEX_DESCRIPTOR_SET_INDEX],
                                    shaders::sprite_frag::DescriptorSet1Args {
                                        tex: &sprite.image_view,
                                    },
                                )
                                .unwrap();

                            let material_index = per_material_descriptor_sets.len() as u32;
                            per_material_descriptor_sets.push(descriptor_set);
                            material_index
                        });

                    //
                    // Assign a batch index for this draw.
                    //  - Opaque sprites can be batched by material alone
                    //  - Transparent sprites can be batched by material + z depth
                    //
                    // We defer creating render nodes to later when we walk through every view. We will
                    // sort all the view nodes by batch index, allowing us to place the vertex data
                    // within a batch contiguously in vertex/index buffers. This way they can be
                    // rendered with a single draw call
                    //
                    let batch_key = if sprite.color.w >= 1.0 {
                        // Transparent sprites batch by material index and depth
                        BatchKey {
                            material_index,
                            depth: None,
                        }
                    } else {
                        // Transparent sprites batch by material index and depth
                        BatchKey {
                            material_index,
                            depth: Some(DecimalF32(sprite.position.z)),
                        }
                    };

                    let batch_index = *batch_key_lookup.entry(batch_key).or_insert_with(|| {
                        let batch_index = batch_count;
                        batch_count += 1;
                        batch_index
                    });

                    batch_indices.push(Some(PerNodeData {
                        batch_index,
                        material_index,
                    }));
                } else {
                    // sprite node that was not extracted, can occur if the asset was not loaded
                    batch_indices.push(None);
                }
            }
        }

        let mut vertex_data = Vec::<SpriteVertex>::default();
        let mut index_data = Vec::<u16>::default();

        let sorted_view_nodes = {
            profiling::scope!("create sorted view nodes");

            let mut sorted_view_nodes =
                Vec::with_capacity(view_packet.render_object_instances().len());

            for sprite in view_packet.render_object_instances().iter() {
                let render_object_instance_id = sprite.render_object_instance_id;
                if let Some(batch_index) = &batch_indices[render_object_instance_id as usize] {
                    sorted_view_nodes.push((batch_index.batch_index, render_object_instance_id));
                }
            }

            {
                profiling::scope!("sort by key");
                sorted_view_nodes.sort_by_key(|x| x.0);
            }

            sorted_view_nodes
        };

        let mut previous_batch_index = 0;
        let mut last_submit_node: Option<(RenderPhaseIndex, SubmitNodeId)> = None;

        {
            profiling::scope!("create draw calls");

            for (batch_index, frame_node_index) in sorted_view_nodes {
                const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;

                if let Some(sprite) = context
                    .render_object_instances_data()
                    .get(frame_node_index as usize)
                {
                    //
                    // If the vertex count exceeds what a u16 index buffer support, start a new draw call
                    //
                    let mut vertex_count = (last_submit_node
                        .map(|x| {
                            view_submit_packet.get_submit_node_data_from_render_phase(x.0, x.1)
                        })
                        .map(|submit_node| submit_node.index_count.load(Ordering::Relaxed))
                        .unwrap_or(0)
                        / 6)
                        * 4;

                    if last_submit_node.is_none()
                        || vertex_count + 4 > u16::MAX as u32
                        || batch_index != previous_batch_index
                    {
                        let material_index = batch_indices[frame_node_index as usize]
                            .as_ref()
                            .unwrap()
                            .material_index;

                        let texture_descriptor_set =
                            per_material_descriptor_sets[material_index as usize].clone();

                        let index_count = AtomicU32::new(0); // This will be incremented as sprites are added.
                        let vertex_data_offset_index = vertex_data.len() as u32;
                        let index_data_offset_index = index_data.len() as u32;

                        let submit_node_data = SpriteDrawCall {
                            texture_descriptor_set: Some(texture_descriptor_set),
                            vertex_data_offset_index,
                            index_data_offset_index,
                            index_count,
                        };

                        let (render_phase_index, distance) = if sprite.color.w >= 1.0 {
                            // non-transparent can just batch by material
                            (OpaqueRenderPhase::render_phase_index(), 0.)
                        } else {
                            // transparent must be ordered by distance
                            let distance = (view.eye_position().z - sprite.position.z).abs();
                            (TransparentRenderPhase::render_phase_index(), distance)
                        };

                        last_submit_node = Some((
                            render_phase_index,
                            view_submit_packet.push_submit_node_into_render_phase(
                                render_phase_index,
                                submit_node_data,
                                batch_index,
                                distance,
                            ),
                        ));

                        previous_batch_index = batch_index;
                        vertex_count = 0;
                    }

                    let matrix = glam::Mat4::from_scale_rotation_translation(
                        glam::Vec3::new(
                            sprite.texture_size.x * sprite.scale.x,
                            sprite.texture_size.y * sprite.scale.y,
                            1.0,
                        ),
                        sprite.rotation,
                        sprite.position,
                    );

                    let color: [f32; 4] = sprite.color.into();
                    let color_u8 = [
                        (color[0].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                        (color[1].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                        (color[2].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                        (color[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                    ];

                    for vertex in &QUAD_VERTEX_LIST {
                        let transformed_pos = matrix.transform_point3(vertex.pos.into());
                        vertex_data.push(SpriteVertex {
                            pos: transformed_pos.into(),
                            tex_coord: vertex.tex_coord,
                            color: color_u8,
                        });
                    }

                    for &index in &QUAD_INDEX_LIST {
                        index_data.push(index + vertex_count as u16);
                    }

                    //
                    // Update the draw call to include the new data
                    //

                    let current_draw_call_data = &last_submit_node
                        .map(|x| {
                            view_submit_packet.get_submit_node_data_from_render_phase(x.0, x.1)
                        })
                        .unwrap();

                    current_draw_call_data
                        .index_count
                        .fetch_add(6, Ordering::Relaxed);
                }
            }
        }

        //
        // If we have vertex data, create the vertex buffer
        //

        let vertex_buffer = if !last_submit_node.is_none() {
            let vertex_buffer_size =
                vertex_data.len() as u64 * std::mem::size_of::<SpriteVertex>() as u64;

            let vertex_buffer = self
                .device_context
                .create_buffer(&RafxBufferDef {
                    size: vertex_buffer_size,
                    memory_usage: RafxMemoryUsage::CpuToGpu,
                    resource_type: RafxResourceType::VERTEX_BUFFER,
                    ..Default::default()
                })
                .unwrap();

            vertex_buffer
                .copy_to_host_visible_buffer(vertex_data.as_slice())
                .unwrap();

            Some(dyn_resource_allocator_set.insert_buffer(vertex_buffer))
        } else {
            None
        };

        per_view_submit_data.vertex_buffer = vertex_buffer.clone();

        //
        // If we have index data, create the index buffer
        //
        let index_buffer = if !last_submit_node.is_none() {
            let index_buffer_size = index_data.len() as u64 * std::mem::size_of::<u16>() as u64;

            let index_buffer = self
                .device_context
                .create_buffer(&RafxBufferDef {
                    size: index_buffer_size,
                    memory_usage: RafxMemoryUsage::CpuToGpu,
                    resource_type: RafxResourceType::INDEX_BUFFER,
                    ..Default::default()
                })
                .unwrap();

            index_buffer
                .copy_to_host_visible_buffer(index_data.as_slice())
                .unwrap();

            Some(dyn_resource_allocator_set.insert_buffer(index_buffer))
        } else {
            None
        };

        per_view_submit_data.index_buffer = index_buffer.clone();

        view_submit_packet
            .per_view_submit_data()
            .set(per_view_submit_data);
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = SpriteRenderFeatureTypes;
    type SubmitPacketDataT = SpriteRenderFeatureTypes;
}
