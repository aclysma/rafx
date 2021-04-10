use rafx::render_feature_write_job_prelude::*;

use super::TileLayerVertex;
use rafx::api::RafxPrimitiveTopology;
use rafx::framework::{VertexDataLayout, VertexDataSetLayout};

lazy_static::lazy_static! {
    pub static ref TILE_LAYER_VERTEX_LAYOUT : VertexDataSetLayout = {
        use rafx::api::RafxFormat;
        VertexDataLayout::build_vertex_layout(&TileLayerVertex::default(), |builder, vertex| {
            builder.add_member(&vertex.position, "POSITION", RafxFormat::R32G32B32_SFLOAT);
            builder.add_member(&vertex.uv, "TEXCOORD", RafxFormat::R32G32_SFLOAT);
        }).into_set(RafxPrimitiveTopology::TriangleList)
    };
}

use super::TileLayerRenderNode;
use rafx::api::{RafxIndexBufferBinding, RafxIndexType, RafxVertexBufferBinding};
use rafx::framework::{DescriptorSetArc, MaterialPassResource, ResourceArc};
use rafx::nodes::{push_view_indexed_value, RenderViewIndex};

pub struct TileLayerWriteJob {
    visible_render_nodes: Vec<TileLayerRenderNode>,
    per_view_descriptor_sets: Vec<Option<DescriptorSetArc>>,
    tile_layer_material: ResourceArc<MaterialPassResource>,
}

impl TileLayerWriteJob {
    pub fn new(
        tile_layer_material: ResourceArc<MaterialPassResource>,
        visible_render_nodes: Vec<TileLayerRenderNode>,
    ) -> Self {
        TileLayerWriteJob {
            visible_render_nodes,
            per_view_descriptor_sets: Default::default(),
            tile_layer_material,
        }
    }

    pub fn visible_render_nodes(&self) -> &Vec<TileLayerRenderNode> {
        &self.visible_render_nodes
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
}

impl WriteJob for TileLayerWriteJob {
    fn apply_setup(
        &self,
        write_context: &mut RenderJobWriteContext,
        _view: &RenderView,
        render_phase_index: RenderPhaseIndex,
    ) -> RafxResult<()> {
        profiling::scope!(super::apply_setup_scope);

        let command_buffer = &write_context.command_buffer;

        let pipeline = write_context
            .resource_context
            .graphics_pipeline_cache()
            .get_or_create_graphics_pipeline(
                render_phase_index,
                &self.tile_layer_material,
                &write_context.render_target_meta,
                &TILE_LAYER_VERTEX_LAYOUT,
            )
            .unwrap();

        command_buffer.cmd_bind_pipeline(&pipeline.get_raw().pipeline)?;

        Ok(())
    }

    fn render_element(
        &self,
        write_context: &mut RenderJobWriteContext,
        view: &RenderView,
        _render_phase_index: RenderPhaseIndex,
        index: SubmitNodeId,
    ) -> RafxResult<()> {
        profiling::scope!(super::render_element_scope);

        let command_buffer = &write_context.command_buffer;

        // Bind per-pass data (UBO with view/proj matrix, sampler)
        self.per_view_descriptor_sets[view.view_index() as usize]
            .as_ref()
            .unwrap()
            .bind(command_buffer)?;

        self.visible_render_nodes[index as usize]
            .per_layer_descriptor_set
            .bind(command_buffer)?;

        for draw_call in &self.visible_render_nodes[index as usize].draw_call_data {
            command_buffer.cmd_bind_vertex_buffers(
                0,
                &[RafxVertexBufferBinding {
                    buffer: &self.visible_render_nodes[index as usize]
                        .vertex_buffer
                        .get_raw()
                        .buffer,
                    byte_offset: draw_call.vertex_data_offset_in_bytes as u64,
                }],
            )?;

            command_buffer.cmd_bind_index_buffer(&RafxIndexBufferBinding {
                buffer: &self.visible_render_nodes[index as usize]
                    .index_buffer
                    .get_raw()
                    .buffer,
                byte_offset: draw_call.index_data_offset_in_bytes as u64,
                index_type: RafxIndexType::Uint16,
            })?;

            command_buffer.cmd_draw_indexed(draw_call.index_count, 0, 0)?;
        }

        Ok(())
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
