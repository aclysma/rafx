use crate::features::mesh::{MeshRenderFeature, MeshDrawCall, ExtractedFrameNodeMeshData, ExtractedViewNodeMeshData, PreparedViewNodeMeshData};
use renderer_nodes::{RenderFeatureIndex, RenderFeature, SubmitNodeId, FeatureCommandWriter, RenderView};
use crate::RenderJobWriteContext;
use renderer_shell_vulkan::{VkBuffer, VkBufferRaw};
use std::mem::ManuallyDrop;
use renderer_resources::resource_managers::{ResourceArc, PipelineSwapchainInfo, DescriptorSetArc};
use ash::vk;
use ash::version::DeviceV1_0;

pub struct MeshCommandWriter {
    pub pipeline_info: PipelineSwapchainInfo,
    pub descriptor_sets_per_view: Vec<DescriptorSetArc>,
    pub extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub prepared_view_node_mesh_data: Vec<PreparedViewNodeMeshData>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for MeshCommandWriter {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
    ) {
        // println!("render");
        let logical_device = write_context.device_context.device();
        let command_buffer = write_context.command_buffer;
        unsafe {
            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline.get_raw().pipelines[0],
            );
        }
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
        index: SubmitNodeId,
    ) {
        let logical_device = write_context.device_context.device();
        let command_buffer = write_context.command_buffer;

        let view_node_data = &self.prepared_view_node_mesh_data[index as usize];
        let frame_node_data = self.extracted_frame_node_mesh_data[view_node_data.frame_node_index as usize].as_ref().unwrap();

        unsafe {
            // Bind per-pass data (UBO with view/proj matrix, sampler)
            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
                0,
                &[view_node_data.per_view_descriptor.get()],
                &[],
            );

            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
                2,
                &[view_node_data.per_instance_descriptor.get()],
                &[],
            );

            for draw_call in &frame_node_data.draw_calls {
                // Bind per-draw-call data (i.e. texture)
                logical_device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_info.pipeline_layout.get_raw().pipeline_layout,
                    1,
                    &[draw_call.per_material_descriptor.get()],
                    &[],
                );

                logical_device.cmd_bind_vertex_buffers(
                    command_buffer,
                    0, // first binding
                    &[frame_node_data.vertex_buffer.get_raw().buffer],
                    &[draw_call.vertex_buffer_offset_in_bytes as u64], // offsets
                );

                logical_device.cmd_bind_index_buffer(
                    command_buffer,
                    frame_node_data.index_buffer.get_raw().buffer,
                    draw_call.index_buffer_offset_in_bytes as u64, // offset
                    vk::IndexType::UINT16,
                );

                logical_device.cmd_draw_indexed(
                    command_buffer,
                    draw_call.index_buffer_size_in_bytes / 2, //sizeof(u16)
                    1,
                    0,
                    0,
                    0,
                );
            }
        }
    }

    fn revert_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
        view: &RenderView,
    ) {

    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}
