use rafx::render_feature_prepare_job_predule::*;

use super::{SpriteVertex, WriteJobImpl};
use crate::phases::OpaqueRenderPhase;
use crate::phases::TransparentRenderPhase;
use fnv::FnvHashMap;
use rafx::api::{RafxBufferDef, RafxMemoryUsage, RafxResourceType};
use rafx::base::DecimalF32;
use rafx::framework::{ImageViewResource, MaterialPassResource, ResourceArc};

/// Per-pass "global" data
pub type SpriteUniformBufferObject = shaders::sprite_vert::ArgsUniform;

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

#[derive(Debug)]
pub struct ExtractedSpriteData {
    pub position: glam::Vec3,
    pub texture_size: glam::Vec2,
    pub scale: glam::Vec2,
    pub rotation: glam::Quat,
    pub color: glam::Vec4,
    pub image_view: ResourceArc<ImageViewResource>,
}

pub struct PrepareJobImpl {
    extracted_frame_node_sprite_data: Vec<Option<ExtractedSpriteData>>,
    sprite_material: ResourceArc<MaterialPassResource>,
}

impl PrepareJobImpl {
    pub(super) fn new(
        extracted_sprite_data: Vec<Option<ExtractedSpriteData>>,
        sprite_material: ResourceArc<MaterialPassResource>,
    ) -> Self {
        PrepareJobImpl {
            extracted_frame_node_sprite_data: extracted_sprite_data,
            sprite_material,
        }
    }
}

impl PrepareJob for PrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes) {
        profiling::scope!(super::prepare_scope);

        let mut writer = Box::new(WriteJobImpl::new(self.sprite_material.clone()));

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        let descriptor_set_layouts = self.sprite_material.get_raw().descriptor_set_layouts;

        //
        // Create per-view descriptor sets
        //
        for view in views {
            let layout = &self.sprite_material.get_raw().descriptor_set_layouts
                [shaders::sprite_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX];
            let descriptor_set = descriptor_set_allocator
                .create_descriptor_set_with_writer(
                    &*layout,
                    shaders::sprite_vert::DescriptorSet0Args {
                        uniform_buffer: &shaders::sprite_vert::ArgsUniform {
                            mvp: view.view_proj().to_cols_array_2d(),
                        },
                    },
                )
                .unwrap();

            writer.push_per_view_descriptor_set(view.view_index(), descriptor_set);
        }

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
            Vec::<Option<PerNodeData>>::with_capacity(self.extracted_frame_node_sprite_data.len());

        {
            profiling::scope!("batch assignment");

            for sprite in &self.extracted_frame_node_sprite_data {
                if let Some(sprite) = sprite {
                    //
                    // First, get or create the descriptor set for the material
                    //
                    //TODO: Cache and reuse where image/material is the same
                    let material_index = *material_lookup
                        .entry(sprite.image_view.clone())
                        .or_insert_with(|| {
                            let descriptor_set = descriptor_set_allocator
                                .create_descriptor_set_with_writer(
                                    &descriptor_set_layouts
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

        let mut submit_nodes = FeatureSubmitNodes::default();
        let mut vertex_data = Vec::<SpriteVertex>::default();
        let mut index_data = Vec::<u16>::default();

        for view in views {
            profiling::scope!("process view");
            if let Some(view_nodes) = frame_packet.view_nodes(view, self.feature_index()) {
                let mut sorted_view_nodes = Vec::with_capacity(
                    frame_packet.view_node_count(view, self.feature_index()) as usize,
                );
                for view_node in view_nodes {
                    if let Some(batch_index) = &batch_indices[view_node.frame_node_index() as usize]
                    {
                        sorted_view_nodes
                            .push((batch_index.batch_index, view_node.frame_node_index()));
                    }
                }

                {
                    profiling::scope!("sort view nodes");
                    sorted_view_nodes.sort_by_key(|x| x.0);
                }

                let mut view_submit_nodes =
                    ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());

                {
                    let mut previous_batch_index = 0;

                    profiling::scope!("write buffer data");
                    for (batch_index, frame_node_index) in sorted_view_nodes {
                        const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;
                        let sprite =
                            &self.extracted_frame_node_sprite_data[frame_node_index as usize];

                        if let Some(sprite) = sprite {
                            //
                            // If the vertex count exceeds what a u16 index buffer support, start a new draw call
                            //
                            let mut vertex_count = (writer
                                .draw_calls()
                                .last()
                                .map(|x| x.index_count)
                                .unwrap_or(0)
                                / 6)
                                * 4;

                            if writer.draw_calls().is_empty()
                                || vertex_count + 4 > u16::MAX as u32
                                || batch_index != previous_batch_index
                            {
                                let submit_node_id = writer.draw_calls().len() as u32;

                                if sprite.color.w >= 1.0 {
                                    // non-transparent can just batch by material
                                    view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(
                                        submit_node_id,
                                        batch_index,
                                        0.0,
                                    );
                                } else {
                                    // transparent must be ordered by distance
                                    let distance =
                                        (view.eye_position().z - sprite.position.z).abs();
                                    view_submit_nodes.add_submit_node::<TransparentRenderPhase>(
                                        submit_node_id,
                                        batch_index,
                                        distance,
                                    );
                                }

                                let material_index = batch_indices[frame_node_index as usize]
                                    .as_ref()
                                    .unwrap()
                                    .material_index;

                                writer.push_draw_call(
                                    vertex_data.len(),
                                    index_data.len(),
                                    per_material_descriptor_sets[material_index as usize].clone(),
                                );

                                previous_batch_index = batch_index;
                                vertex_count = 0;
                            }

                            let current_draw_call_data =
                                writer.draw_calls_mut().last_mut().unwrap();

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
                            current_draw_call_data.index_count += 6;
                        }
                    }
                }

                submit_nodes.add_submit_nodes_for_view(&view, view_submit_nodes);
            }
        }

        //
        // If we have vertex data, create the vertex buffer
        //
        let dyn_resource_allocator = prepare_context
            .resource_context
            .create_dyn_resource_allocator_set();

        let vertex_buffer = if !writer.draw_calls().is_empty() {
            let vertex_buffer_size =
                vertex_data.len() as u64 * std::mem::size_of::<SpriteVertex>() as u64;

            let vertex_buffer = prepare_context
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

            Some(dyn_resource_allocator.insert_buffer(vertex_buffer))
        } else {
            None
        };

        writer.set_vertex_buffer(vertex_buffer);

        //
        // If we have index data, create the index buffer
        //
        let index_buffer = if !writer.draw_calls().is_empty() {
            let index_buffer_size = index_data.len() as u64 * std::mem::size_of::<u16>() as u64;

            let index_buffer = prepare_context
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

            Some(dyn_resource_allocator.insert_buffer(index_buffer))
        } else {
            None
        };

        writer.set_index_buffer(index_buffer);

        log::trace!("sprite draw calls: {}", writer.draw_calls().len());

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
