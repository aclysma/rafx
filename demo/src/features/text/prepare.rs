use rafx::render_feature_prepare_job_predule::*;

use super::FontAtlasCache;
use super::WriteJobImpl;
use super::{RenderFeatureType, TextDrawCommand};
use crate::assets::font::FontAsset;
use crate::phases::UiRenderPhase;
use distill::loader::LoadHandle;
use fnv::FnvHashMap;
use rafx::api::RafxBufferDef;
use rafx::framework::{MaterialPassResource, ResourceArc};

pub type TextUniformBufferObject = shaders::text_vert::PerViewUboUniform;

pub struct PrepareJobImpl {
    text_material_pass: ResourceArc<MaterialPassResource>,
    text_draw_commands: Vec<TextDrawCommand>,
    font_assets: FnvHashMap<LoadHandle, FontAsset>,
}

impl PrepareJobImpl {
    pub(super) fn new(
        text_material_pass: ResourceArc<MaterialPassResource>,
        text_draw_commands: Vec<TextDrawCommand>,
        font_assets: FnvHashMap<LoadHandle, FontAsset>,
    ) -> Self {
        PrepareJobImpl {
            text_material_pass,
            text_draw_commands,
            font_assets,
        }
    }
}

impl<'a> PrepareJob for PrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes) {
        profiling::scope!(super::prepare_scope);

        let dyn_resource_allocator = prepare_context
            .resource_context
            .create_dyn_resource_allocator_set();

        let mut font_atlas_cache = prepare_context
            .render_resources
            .fetch_mut::<FontAtlasCache>();

        let draw_vertices_result = font_atlas_cache
            .generate_vertices(
                &self.text_draw_commands,
                &self.font_assets,
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

        let mut submit_nodes = FeatureSubmitNodes::default();

        let mut writer = Box::new(WriteJobImpl::new(
            self.text_material_pass.clone(),
            draw_vertices_result.draw_call_metas,
            draw_vertices_result.image_updates,
            draw_vertices_result.draw_call_buffer_data.len(),
        ));

        if !writer.draw_call_metas().is_empty() {
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

                writer.push_per_font_descriptor_set(per_font_descriptor_set);
            }

            //
            // Create per-view descriptor sets (i.e. a projection matrix)
            //
            for view in views
                .iter()
                .filter(|view| view.feature_is_relevant::<RenderFeatureType>())
            {
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

                writer.push_per_view_descriptor_set(view.view_index(), per_view_descriptor_set);
            }
        }

        //
        // Update the vertex buffers
        //

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

            writer.push_buffers(vertex_buffer, index_buffer);
        }

        //
        // Submit a single node for each view
        // TODO: Submit separate nodes for transparency/text positioned in 3d
        //
        for view in views
            .iter()
            .filter(|view| view.feature_is_relevant::<RenderFeatureType>())
        {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());

            for (submit_node_id, draw_call) in writer.draw_call_metas().iter().enumerate() {
                view_submit_nodes.add_submit_node::<UiRenderPhase>(
                    submit_node_id as u32,
                    0,
                    draw_call.z_position,
                );
            }

            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        super::render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }
}
