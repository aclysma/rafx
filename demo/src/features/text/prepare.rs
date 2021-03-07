use super::write::TextCommandWriter;
use crate::features::text::write::TextDrawCallBuffers;
use crate::features::text::{
    ExtractedTextData, FontAtlasCache, TextRenderFeature, TextUniformBufferObject,
};
use crate::phases::UiRenderPhase;
use rafx::api::RafxBufferDef;
use rafx::framework::{MaterialPassResource, ResourceArc};
use rafx::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PrepareJob, RenderFeature,
    RenderFeatureIndex, RenderJobPrepareContext, RenderView, ViewSubmitNodes,
};

pub struct TextPrepareJobImpl {
    text_material_pass: ResourceArc<MaterialPassResource>,
    extracted_text_data: ExtractedTextData,
}

impl TextPrepareJobImpl {
    pub(super) fn new(
        text_material_pass: ResourceArc<MaterialPassResource>,
        extracted_text_data: ExtractedTextData,
    ) -> Self {
        TextPrepareJobImpl {
            text_material_pass,
            extracted_text_data,
        }
    }
}

impl<'a> PrepareJob for TextPrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes) {
        profiling::scope!("Text Prepare");

        let dyn_resource_allocator = prepare_context
            .resource_context
            .create_dyn_resource_allocator_set();

        let mut font_atlas_cache = prepare_context
            .render_resources
            .fetch_mut::<FontAtlasCache>();
        let draw_vertices_result = font_atlas_cache
            .generate_vertices(
                &self.extracted_text_data.text_draw_commands,
                &self.extracted_text_data.font_assets,
                &dyn_resource_allocator,
            )
            .unwrap();

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        // Get the layouts for both descriptor sets
        let per_view_descriptor_set_layout =
            &self.text_material_pass.get_raw().descriptor_set_layouts
                [shaders::text_vert::PER_VIEW_DATA_DESCRIPTOR_SET_INDEX];

        let per_font_descriptor_set_layout =
            &self.text_material_pass.get_raw().descriptor_set_layouts
                [shaders::text_frag::TEX_DESCRIPTOR_SET_INDEX];

        let mut per_font_descriptor_sets = Vec::default();
        let mut per_view_descriptor_sets = Vec::default();
        let mut submit_nodes = FeatureSubmitNodes::default();

        if !draw_vertices_result.draw_call_metas.is_empty() {
            //
            // Create per-font descriptor sets (i.e. a font atlas texture)
            //
            for image in &draw_vertices_result.font_atlas_images {
                let per_font_descriptor_set = descriptor_set_allocator
                    .create_descriptor_set(
                        per_font_descriptor_set_layout,
                        shaders::text_frag::DescriptorSet0Args { tex: &image },
                    )
                    .unwrap();

                per_font_descriptor_sets.push(per_font_descriptor_set.clone());
            }

            //
            // Create per-view descriptor sets (i.e. a projection matrix) and add a submit node per
            // each view
            //
            for view in views {
                //
                // Setup the vertex shader descriptor set
                //
                let proj = glam::Mat4::orthographic_rh(
                    0.0,
                    view.extents_width() as f32,
                    view.extents_height() as f32,
                    0.0,
                    -1000.0,
                    100.0,
                );

                let text_view = TextUniformBufferObject {
                    view_proj: proj.to_cols_array_2d(),
                };

                let per_view_descriptor_set = descriptor_set_allocator
                    .create_descriptor_set(
                        per_view_descriptor_set_layout,
                        shaders::text_vert::DescriptorSet1Args {
                            per_view_data: &text_view,
                        },
                    )
                    .unwrap();

                // Grow the array if necessary
                per_view_descriptor_sets.resize(
                    per_view_descriptor_sets
                        .len()
                        .max(view.view_index() as usize + 1),
                    None,
                );

                per_view_descriptor_sets[view.view_index() as usize] =
                    Some(per_view_descriptor_set.clone());
            }
        }

        //
        // Update the vertex buffers
        //
        let mut draw_call_buffers =
            Vec::with_capacity(draw_vertices_result.draw_call_buffer_data.len());
        for draw_call in draw_vertices_result.draw_call_buffer_data {
            let vertex_buffer = prepare_context
                .device_context
                .create_buffer(&RafxBufferDef::for_staging_vertex_buffer_data(
                    &draw_call.vertices,
                ))
                .unwrap();

            vertex_buffer
                .copy_to_host_visible_buffer(draw_call.vertices.as_slice())
                .unwrap();

            let vertex_buffer = dyn_resource_allocator.insert_buffer(vertex_buffer);

            let index_buffer = prepare_context
                .device_context
                .create_buffer(&RafxBufferDef::for_staging_index_buffer_data(
                    &draw_call.indices,
                ))
                .unwrap();

            index_buffer
                .copy_to_host_visible_buffer(draw_call.indices.as_slice())
                .unwrap();

            let index_buffer = dyn_resource_allocator.insert_buffer(index_buffer);

            draw_call_buffers.push(TextDrawCallBuffers {
                vertex_buffer,
                index_buffer,
            })
        }

        //
        // Submit a single node for each view
        // TODO: Submit separate nodes for transparency/text positioned in 3d
        //
        for view in views {
            for (i, draw_call) in draw_vertices_result.draw_call_metas.iter().enumerate() {
                let mut view_submit_nodes =
                    ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());
                view_submit_nodes.add_submit_node::<UiRenderPhase>(
                    i as u32,
                    0,
                    draw_call.z_position,
                );
                submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
            }
        }

        let writer = Box::new(TextCommandWriter {
            draw_call_buffers,
            draw_call_metas: draw_vertices_result.draw_call_metas,
            text_material_pass: self.text_material_pass,
            per_font_descriptor_sets,
            per_view_descriptor_sets,
            image_updates: draw_vertices_result.image_updates,
        });

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        TextRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        TextRenderFeature::feature_index()
    }
}
