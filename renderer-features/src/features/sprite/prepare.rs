use crate::phases::draw_transparent::DrawTransparentRenderPhase;
use renderer_base::{RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex, FramePacket, DefaultPrepareJobImpl, PerFrameNode, PerViewNode, RenderFeature};
use crate::features::sprite::{SpriteRenderFeature, ExtractedSpriteData, QUAD_VERTEX_LIST, QUAD_INDEX_LIST, SpriteDrawCall, SpriteVertex};
use crate::phases::draw_opaque::DrawOpaqueRenderPhase;
use glam::Vec3;
use super::SpriteCommandWriter;
use crate::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer_shell_vulkan::{VkBuffer, VkDeviceContext};
use ash::vk;
use std::mem::ManuallyDrop;
use renderer_assets::resource_managers::{PipelineSwapchainInfo, DescriptorSetArc};

pub struct SpritePrepareJobImpl {
    device_context: VkDeviceContext,
    pipeline_info: PipelineSwapchainInfo,
    descriptor_set_per_pass: DescriptorSetArc,
    extracted_sprite_data: Vec<Option<ExtractedSpriteData>>,

    draw_calls: Vec<SpriteDrawCall>,
    vertex_list: Vec<SpriteVertex>,
    index_list: Vec<u16>,
}

impl SpritePrepareJobImpl {
    pub(super) fn new(
        device_context: VkDeviceContext,
        pipeline_info: PipelineSwapchainInfo,
        descriptor_set_per_pass: DescriptorSetArc,
        extracted_sprite_data: Vec<Option<ExtractedSpriteData>>,
    ) -> Self {
        let sprite_count = extracted_sprite_data.len();
        SpritePrepareJobImpl {
            device_context,
            extracted_sprite_data,
            pipeline_info,
            descriptor_set_per_pass,
            draw_calls: Vec::with_capacity(sprite_count),
            vertex_list: Vec::with_capacity(sprite_count * QUAD_VERTEX_LIST.len()),
            index_list: Vec::with_capacity(sprite_count * QUAD_INDEX_LIST.len()),
        }
    }
}

impl DefaultPrepareJobImpl<RenderJobPrepareContext, RenderJobWriteContext> for SpritePrepareJobImpl {
    fn prepare_begin(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        _views: &[&RenderView],
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {
        for sprite in &self.extracted_sprite_data {
            if let Some(sprite) = sprite {
                let draw_call = SpriteDrawCall {
                    index_buffer_first_element: 0,
                    index_buffer_count: 4,
                    texture_descriptor_set: sprite.texture_descriptor_set.clone(),
                };

                const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;

                let matrix = glam::Mat4::from_translation(sprite.position)
                    * glam::Mat4::from_rotation_z(sprite.rotation * DEG_TO_RAD)
                    * glam::Mat4::from_scale(glam::Vec3::new(
                    sprite.texture_size.x() * sprite.scale,
                    sprite.texture_size.y() * sprite.scale,
                    1.0,
                ));

                let vertex_buffer_first_element = self.vertex_list.len() as u16;

                for vertex in &QUAD_VERTEX_LIST {
                    //let pos = vertex.pos;
                    let transformed_pos = matrix.transform_point3(vertex.pos.into());

                    self.vertex_list.push(SpriteVertex {
                        pos: transformed_pos.truncate().into(),
                        tex_coord: vertex.tex_coord,
                        //color: [255, 255, 255, 255]
                    });
                }

                let index_buffer_first_element = self.index_list.len() as u16;
                for index in &QUAD_INDEX_LIST {
                    self.index_list.push((*index + vertex_buffer_first_element));
                }

                let draw_call = SpriteDrawCall {
                    index_buffer_first_element,
                    index_buffer_count: QUAD_INDEX_LIST.len() as u16,
                    texture_descriptor_set: sprite.texture_descriptor_set.clone(),
                };

                self.draw_calls.push(draw_call);
            }
        }
    }

    fn prepare_frame_node(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        _frame_node: PerFrameNode,
        frame_node_index: u32,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) {

    }

    fn prepare_view_node(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        view: &RenderView,
        view_node: PerViewNode,
        view_node_index: u32,
        submit_nodes: &mut ViewSubmitNodes,
    ) {
        // Use the frame node index as the submit ID since we don't have any view-specific data
        // to cache
        let frame_node_index = view_node.frame_node_index();

        // This can read per-frame and per-view data
        if let Some(extracted_data) = &self.extracted_sprite_data[frame_node_index as usize] {
            if extracted_data.alpha >= 1.0 {
                submit_nodes.add_submit_node::<DrawOpaqueRenderPhase>(frame_node_index, 0, 0.0);
            } else {
                let distance_from_camera = Vec3::length(extracted_data.position - view.eye_position());
                submit_nodes.add_submit_node::<DrawTransparentRenderPhase>(
                    frame_node_index,
                    0,
                    distance_from_camera,
                );
            }
        }
    }

    fn prepare_view_finalize(
        &mut self,
        prepare_context: &RenderJobPrepareContext,
        _view: &RenderView,
        _submit_nodes: &mut ViewSubmitNodes,
    ) {

    }

    fn prepare_frame_finalize(
        self,
        prepare_context: &RenderJobPrepareContext,
        _submit_nodes: &mut FeatureSubmitNodes,
    ) -> Box<dyn FeatureCommandWriter<RenderJobWriteContext>> {
        //TODO: indexes are u16 so we may need to produce more than one set of buffers
        let mut vertex_buffers = Vec::with_capacity(1);
        let mut index_buffers = Vec::with_capacity(1);

        if self.draw_calls.len() > 0 {
            //TODO: It's likely unnecessary to put all the data into a Vec and then copy it into the buffer. We could
            // write to the buffer to begin with
            let vertex_buffer = {
                let vertex_buffer_size =
                    self.vertex_list.len() as u64 * std::mem::size_of::<SpriteVertex>() as u64;
                let mut vertex_buffer = VkBuffer::new(
                    &self.device_context,
                    vk_mem::MemoryUsage::CpuToGpu,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    vertex_buffer_size,
                ).unwrap();

                vertex_buffer.write_to_host_visible_buffer(self.vertex_list.as_slice()).unwrap();

                let vertex_buffer = prepare_context.dyn_resource_lookups.insert_buffer(vertex_buffer);
                vertex_buffer
            };

            let index_buffer = {
                let index_buffer_size = self.index_list.len() as u64 * std::mem::size_of::<u16>() as u64;
                let mut index_buffer = VkBuffer::new(
                    &self.device_context,
                    vk_mem::MemoryUsage::CpuToGpu,
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                    index_buffer_size,
                ).unwrap();

                index_buffer.write_to_host_visible_buffer(self.index_list.as_slice()).unwrap();

                let index_buffer = prepare_context.dyn_resource_lookups.insert_buffer(index_buffer);
                index_buffer
            };

            vertex_buffers.push(vertex_buffer);
            index_buffers.push(index_buffer);
        }

        Box::new(SpriteCommandWriter {
            draw_calls: self.draw_calls,
            vertex_buffers,
            index_buffers,
            pipeline_info: self.pipeline_info,
            descriptor_set_per_pass: self.descriptor_set_per_pass,
        })
    }

    fn feature_debug_name(&self) -> &'static str {
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}
