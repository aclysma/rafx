use crate::features::mesh::{
    ExtractedFrameNodeMeshData, MeshRenderFeature, PreparedSubmitNodeMeshData,
};
use crate::render_contexts::RenderJobWriteContext;
use ash::version::DeviceV1_0;
use ash::vk;
use rafx::nodes::{
    FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderPhaseIndex, RenderView,
    SubmitNodeId,
};

pub struct MeshCommandWriter {
    pub(super) extracted_frame_node_mesh_data: Vec<Option<ExtractedFrameNodeMeshData>>,
    pub(super) prepared_submit_node_mesh_data: Vec<PreparedSubmitNodeMeshData>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for MeshCommandWriter {
    fn apply_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
        // println!("render");
        // let logical_device = write_context.device_context.device();
        // let command_buffer = write_context.command_buffer;
        // unsafe {
        //     logical_device.cmd_bind_pipeline(
        //         command_buffer,
        //         vk::PipelineBindPoint::GRAPHICS,
        //         self.pipeline_info.get_raw().pipelines[0],
        //     );
        // }
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) {
        let logical_device = write_context.device_context.device();
        let command_buffer = write_context.command_buffer;

        let render_node_data = &self.prepared_submit_node_mesh_data[index as usize];
        let frame_node_data: &ExtractedFrameNodeMeshData = self.extracted_frame_node_mesh_data
            [render_node_data.frame_node_index as usize]
            .as_ref()
            .unwrap();

        unsafe {
            // Always valid, we don't generate render nodes for mesh parts that are None
            let mesh_part = &frame_node_data.mesh_asset.inner.mesh_parts
                [render_node_data.mesh_part_index]
                .as_ref()
                .unwrap();

            let pipeline = write_context
                .resource_context
                .graphics_pipeline_cache()
                .get_or_create_graphics_pipeline(
                    &render_node_data.material_pass.material_pass_resource,
                    &write_context.renderpass,
                    &write_context.framebuffer_meta,
                    &*crate::assets::gltf::MESH_VERTEX_LAYOUT,
                )
                .unwrap();

            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.get_raw().pipelines[write_context.subpass_index as usize],
            );

            // frag shader per-view data, not present during shadow pass
            if let Some(per_view_descriptor_set) = &render_node_data.per_view_descriptor_set {
                // Bind per-pass data (UBO with view/proj matrix, sampler)
                logical_device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                    super::PER_VIEW_DESCRIPTOR_SET_INDEX,
                    &[per_view_descriptor_set.get()],
                    &[],
                );
            };

            // frag shader material data, not present during shadow pass
            if let Some(per_material_descriptor_set) = &render_node_data.per_material_descriptor_set
            {
                logical_device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                    super::PER_MATERIAL_DESCRIPTOR_SET_INDEX,
                    &[per_material_descriptor_set.get()], // pass 0, descriptor set index 1
                    &[],
                );
            }

            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.get_raw().pipeline_layout.get_raw().pipeline_layout,
                super::PER_INSTANCE_DESCRIPTOR_SET_INDEX,
                &[render_node_data.per_instance_descriptor_set.get()],
                &[],
            );

            logical_device.cmd_bind_vertex_buffers(
                command_buffer,
                0, // first binding
                &[frame_node_data
                    .mesh_asset
                    .inner
                    .vertex_buffer
                    .get_raw()
                    .buffer
                    .buffer],
                &[mesh_part.vertex_buffer_offset_in_bytes as u64], // offsets
            );

            logical_device.cmd_bind_index_buffer(
                command_buffer,
                frame_node_data
                    .mesh_asset
                    .inner
                    .index_buffer
                    .get_raw()
                    .buffer
                    .buffer,
                mesh_part.index_buffer_offset_in_bytes as u64, // offset
                vk::IndexType::UINT16,
            );

            logical_device.cmd_draw_indexed(
                command_buffer,
                mesh_part.index_buffer_size_in_bytes / 2, //sizeof(u16)
                1,
                0,
                0,
                0,
            );
        }
    }

    fn revert_setup(
        &self,
        _write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
    }

    fn feature_debug_name(&self) -> &'static str {
        MeshRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        MeshRenderFeature::feature_index()
    }
}
