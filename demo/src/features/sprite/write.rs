rafx::declare_render_feature_write_job!();

use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Default)]
#[repr(C)]
pub struct SpriteVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
    pub color: [u8; 4],
}

lazy_static::lazy_static! {
    pub static ref SPRITE_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;

        VertexDataLayout::build_vertex_layout(&SpriteVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.pos, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.tex_coord, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
            builder.add_member(&vertex.color, "COLOR", RafxFormat::R8G8B8A8_UNORM);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

use rafx::api::{RafxIndexBufferBinding, RafxIndexType, RafxVertexBufferBinding};
use rafx::framework::{BufferResource, DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::nodes::{push_view_indexed_value, RenderViewIndex};

#[derive(Debug)]
pub struct SpriteDrawCall {
    pub texture_descriptor_set: DescriptorSetArc,
    pub vertex_data_offset_index: u32,
    pub index_data_offset_index: u32,
    pub index_count: u32,
}

pub struct FeatureCommandWriterImpl {
    vertex_buffer: Option<ResourceArc<BufferResource>>,
    index_buffer: Option<ResourceArc<BufferResource>>,
    draw_calls: Vec<SpriteDrawCall>,
    per_view_descriptor_sets: Vec<Option<DescriptorSetArc>>,
    sprite_material: ResourceArc<MaterialPassResource>,
}

impl FeatureCommandWriterImpl {
    pub fn new(sprite_material: ResourceArc<MaterialPassResource>) -> Self {
        FeatureCommandWriterImpl {
            vertex_buffer: Default::default(),
            index_buffer: Default::default(),
            draw_calls: Default::default(),
            per_view_descriptor_sets: Default::default(),
            sprite_material,
        }
    }

    pub fn push_per_view_descriptor_set(
        &mut self,
        view_index: RenderViewIndex,
        per_view_descriptor_set: DescriptorSetArc,
    ) {
        push_view_indexed_value(
            &mut self.per_view_descriptor_sets,
            view_index,
            per_view_descriptor_set,
        );
    }

    pub fn push_draw_call(
        &mut self,
        vertex_data_offset_index: usize,
        index_data_offset_index: usize,
        texture_descriptor_set: DescriptorSetArc,
    ) {
        self.draw_calls.push(SpriteDrawCall {
            vertex_data_offset_index: vertex_data_offset_index as u32,
            index_data_offset_index: index_data_offset_index as u32,
            index_count: 0,
            texture_descriptor_set,
        });
    }

    pub fn set_vertex_buffer(
        &mut self,
        vertex_buffer: Option<ResourceArc<BufferResource>>,
    ) {
        self.vertex_buffer = vertex_buffer;
    }

    pub fn set_index_buffer(
        &mut self,
        index_buffer: Option<ResourceArc<BufferResource>>,
    ) {
        self.index_buffer = index_buffer;
    }

    pub fn draw_calls(&self) -> &Vec<SpriteDrawCall> {
        &self.draw_calls
    }

    pub fn draw_calls_mut(&mut self) -> &mut Vec<SpriteDrawCall> {
        &mut self.draw_calls
    }
}

impl FeatureCommandWriter for FeatureCommandWriterImpl {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        profiling::scope!(apply_setup_scope);

        let command_buffer = &write_context.command_buffer;

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                render_phase_index,
                &self.sprite_material,
                &write_context.render_target_meta,
                &SPRITE_VERTEX_LAYOUT,
            )
            .unwrap();

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        // Bind per-pass data (UBO with view/proj matrix, sampler)
        self.per_view_descriptor_sets[view.view_index() as usize]
            .as_ref()
            .unwrap()
            .bind(command_buffer)?;

        command_buffer.cmd_bind_vertex_buffers(
            0,
            &[RafxVertexBufferBinding {
                buffer: &self.vertex_buffer.as_ref().unwrap().get_raw().buffer,
                byte_offset: 0,
            }],
        )?;

        command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
            buffer: &self.index_buffer.as_ref().unwrap().get_raw().buffer,
            byte_offset: 0,
            index_type: RafxIndexType::Uint16,
        })?;

        Ok(())
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> RafxResult<()> {
        profiling::scope!(render_element_scope);

        let command_buffer = &write_context.command_buffer;
        let draw_call = &self.draw_calls[index as usize];

        // Bind per-draw-call data (i.e. texture)
        draw_call.texture_descriptor_set.bind(command_buffer)?;

        command_buffer.cmd_draw_indexed(
            draw_call.index_count,
            draw_call.index_data_offset_index,
            draw_call.vertex_data_offset_index as i32,
        )?;

        Ok(())
    }

    fn feature_debug_name(&self) -> &'static str {
        render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }
}
