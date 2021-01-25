use super::SpriteCommandWriter;
use crate::features::sprite::{
    ExtractedSpriteData, SpriteDrawCall, SpriteRenderFeature, SpriteVertex, QUAD_INDEX_LIST,
    QUAD_VERTEX_LIST,
};
use crate::phases::OpaqueRenderPhase;
use crate::phases::TransparentRenderPhase;
use crate::render_contexts::{RenderJobPrepareContext, RenderJobWriteContext};
use fnv::FnvHashMap;
use glam::Vec3;
use rafx::api::{RafxBufferDef, RafxMemoryUsage, RafxResourceType};
use rafx::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PrepareJob, RenderFeature,
    RenderFeatureIndex, RenderView, ViewSubmitNodes,
};
use rafx::resources::{DescriptorSetArc, ImageViewResource, MaterialPassResource, ResourceArc};

pub struct SpritePrepareJob {
    extracted_frame_node_sprite_data: Vec<Option<ExtractedSpriteData>>,
    sprite_material: ResourceArc<MaterialPassResource>,
}

impl SpritePrepareJob {
    pub(super) fn new(
        extracted_sprite_data: Vec<Option<ExtractedSpriteData>>,
        sprite_material: ResourceArc<MaterialPassResource>,
    ) -> Self {
        SpritePrepareJob {
            extracted_frame_node_sprite_data: extracted_sprite_data,
            sprite_material,
        }
    }
}

impl PrepareJob<RenderJobPrepareContext, RenderJobWriteContext> for SpritePrepareJob {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (
        Box<dyn FeatureCommandWriter<RenderJobWriteContext>>,
        FeatureSubmitNodes,
    ) {
        profiling::scope!("Sprite Prepare");

        let mut draw_calls = Vec::<SpriteDrawCall>::default();
        let mut vertex_list = Vec::<SpriteVertex>::default();
        let mut index_list = Vec::<u16>::default();

        let mut per_image_descriptor_sets =
            FnvHashMap::<ResourceArc<ImageViewResource>, DescriptorSetArc>::default();

        //
        // Create per-instance descriptor sets, indexed by frame node
        //
        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        let descriptor_set_layouts = self.sprite_material.get_raw().descriptor_set_layouts;

        for sprite in &self.extracted_frame_node_sprite_data {
            if let Some(sprite) = sprite {
                const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;

                let matrix = glam::Mat4::from_translation(sprite.position)
                    * glam::Mat4::from_rotation_z(sprite.rotation * DEG_TO_RAD)
                    * glam::Mat4::from_scale(glam::Vec3::new(
                        sprite.texture_size.x() * sprite.scale,
                        sprite.texture_size.y() * sprite.scale,
                        1.0,
                    ));

                let vertex_buffer_first_element = vertex_list.len() as u16;

                for vertex in &QUAD_VERTEX_LIST {
                    //let pos = vertex.pos;
                    let transformed_pos = matrix.transform_point3(vertex.pos.into());

                    vertex_list.push(SpriteVertex {
                        pos: transformed_pos.truncate().into(),
                        tex_coord: vertex.tex_coord,
                        //color: [255, 255, 255, 255]
                    });
                }

                let index_buffer_first_element = index_list.len() as u16;
                for index in &QUAD_INDEX_LIST {
                    index_list.push(*index + vertex_buffer_first_element);
                }

                //TODO: Cache and reuse where image/material is the same
                let texture_descriptor_set = per_image_descriptor_sets
                    .entry(sprite.image_view.clone())
                    .or_insert_with(|| {
                        let per_sprite_descriptor_set = descriptor_set_allocator
                            .create_descriptor_set(
                                &descriptor_set_layouts
                                    [shaders::sprite_frag::TEX_DESCRIPTOR_SET_INDEX],
                                shaders::sprite_frag::DescriptorSet1Args {
                                    tex: &sprite.image_view,
                                },
                            )
                            .unwrap();

                        per_sprite_descriptor_set
                    });

                let draw_call = SpriteDrawCall {
                    index_buffer_first_element,
                    index_buffer_count: QUAD_INDEX_LIST.len() as u16,
                    texture_descriptor_set: texture_descriptor_set.clone(),
                };

                draw_calls.push(draw_call);
            }
        }

        let mut per_view_descriptor_sets = Vec::default();

        let extents_width = 900;
        let extents_height = 600;
        let aspect_ratio = extents_width as f32 / extents_height as f32;
        let half_width = 400.0;
        let half_height = 400.0 / aspect_ratio;
        let view_proj = glam::Mat4::orthographic_rh_gl(
            -half_width,
            half_width,
            -half_height,
            half_height,
            -100.0,
            100.0,
        );

        //
        // Add submit nodes per view
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for &view in views {
            if let Some(view_nodes) = frame_packet.view_nodes(view, self.feature_index()) {
                let mut view_submit_nodes =
                    ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());
                for view_node in view_nodes {
                    let frame_node_index = view_node.frame_node_index();
                    if let Some(extracted_data) =
                        &self.extracted_frame_node_sprite_data[frame_node_index as usize]
                    {
                        if extracted_data.alpha >= 1.0 {
                            view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(
                                frame_node_index,
                                0,
                                0.0,
                            );
                        } else {
                            let distance =
                                Vec3::length(extracted_data.position - view.eye_position());
                            view_submit_nodes.add_submit_node::<TransparentRenderPhase>(
                                frame_node_index,
                                0,
                                distance,
                            );
                        }
                    }
                }

                submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
            }

            //TODO: Multi-view support for sprites. Not clear on if we want to do a screen-space view specifically
            // for sprites
            //TODO: Extents is hard-coded
            let layout = &self.sprite_material.get_raw().descriptor_set_layouts
                [shaders::sprite_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX];
            let descriptor_set = descriptor_set_allocator
                .create_descriptor_set(
                    &*layout,
                    shaders::sprite_vert::DescriptorSet0Args {
                        uniform_buffer: &shaders::sprite_vert::ArgsUniform {
                            mvp: view_proj.to_cols_array_2d(),
                        },
                    },
                )
                .unwrap();

            per_view_descriptor_sets.resize(
                per_view_descriptor_sets
                    .len()
                    .max(view.view_index() as usize + 1),
                None,
            );
            per_view_descriptor_sets[view.view_index() as usize] = Some(descriptor_set);
        }

        //TODO: indexes are u16 so we may need to produce more than one set of buffers
        let mut vertex_buffers = Vec::with_capacity(1);
        let mut index_buffers = Vec::with_capacity(1);

        if !draw_calls.is_empty() {
            let dyn_resource_allocator = prepare_context
                .resource_context
                .create_dyn_resource_allocator_set();

            //TODO: It's likely unnecessary to put all the data into a Vec and then copy it into the buffer. We could
            // write to the buffer to begin with
            let vertex_buffer = {
                let vertex_buffer_size =
                    vertex_list.len() as u64 * std::mem::size_of::<SpriteVertex>() as u64;

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
                    .copy_to_host_visible_buffer(vertex_list.as_slice())
                    .unwrap();

                dyn_resource_allocator.insert_buffer(vertex_buffer)
            };

            let index_buffer = {
                let index_buffer_size = index_list.len() as u64 * std::mem::size_of::<u16>() as u64;

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
                    .copy_to_host_visible_buffer(index_list.as_slice())
                    .unwrap();

                dyn_resource_allocator.insert_buffer(index_buffer)
            };

            vertex_buffers.push(vertex_buffer);
            index_buffers.push(index_buffer);
        }

        let writer = Box::new(SpriteCommandWriter {
            draw_calls,
            vertex_buffers,
            index_buffers,
            per_view_descriptor_sets,
            sprite_material: self.sprite_material,
        });

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}
