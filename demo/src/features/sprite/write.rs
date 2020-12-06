use crate::features::sprite::{SpriteDrawCall, SpriteRenderFeature};
use crate::render_contexts::RenderJobWriteContext;
use ash::version::DeviceV1_0;
use ash::vk;
use rafx::nodes::{
    FeatureCommandWriter, RenderFeature, RenderFeatureIndex, RenderPhaseIndex, RenderView,
    SubmitNodeId,
};
use rafx::resources::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};

pub struct SpriteCommandWriter {
    pub vertex_buffers: Vec<ResourceArc<BufferResource>>,
    pub index_buffers: Vec<ResourceArc<BufferResource>>,
    pub draw_calls: Vec<SpriteDrawCall>,
    pub per_view_descriptor_sets: Vec<Option<DescriptorSetArc>>,
    pub sprite_material: ResourceArc<MaterialPassResource>,
}

impl FeatureCommandWriter<RenderJobWriteContext> for SpriteCommandWriter {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
    ) {
        let logical_device = write_context.device_context.device();
        let command_buffer = write_context.command_buffer;

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                &self.sprite_material,
                &write_context.renderpass,
                &write_context.framebuffer_meta,
                &super::SPRITE_VERTEX_LAYOUT,
            )
            .unwrap();

        unsafe {
            logical_device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.get_raw().pipelines[write_context.subpass_index],
            );

            // Bind per-pass data (UBO with view/proj matrix, sampler)
            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.sprite_material
                    .get_raw()
                    .pipeline_layout
                    .get_raw()
                    .pipeline_layout,
                shaders::sprite_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX as u32,
                &[self.per_view_descriptor_sets[view.view_index() as usize]
                    .as_ref()
                    .unwrap()
                    .get()],
                &[],
            );

            logical_device.cmd_bind_vertex_buffers(
                command_buffer,
                0, // first binding
                &[self.vertex_buffers[0].get_raw().buffer.buffer],
                &[0], // offsets
            );

            logical_device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffers[0].get_raw().buffer.buffer,
                0, // offset
                vk::IndexType::UINT16,
            );
        }
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
        let draw_call = &self.draw_calls[index as usize];

        unsafe {
            // Bind per-draw-call data (i.e. texture)
            logical_device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.sprite_material
                    .get_raw()
                    .pipeline_layout
                    .get_raw()
                    .pipeline_layout,
                shaders::sprite_frag::TEX_DESCRIPTOR_SET_INDEX as u32,
                &[draw_call.texture_descriptor_set.get()],
                &[],
            );

            logical_device.cmd_draw_indexed(
                command_buffer,
                draw_call.index_buffer_count as u32,
                1,
                draw_call.index_buffer_first_element as u32,
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
        SpriteRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        SpriteRenderFeature::feature_index()
    }
}
