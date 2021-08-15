use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::UiRenderPhase;
use crate::shaders;
use rafx::api::{RafxBufferDef, RafxDeviceContext};
use rafx::base::resource_map::WriteBorrow;
use rafx::framework::ResourceContext;

pub struct TextPrepareJob<'prepare> {
    font_atlas_cache: TrustCell<WriteBorrow<'prepare, FontAtlasCache>>,
    resource_context: ResourceContext,
    device_context: RafxDeviceContext,
}

impl<'prepare> TextPrepareJob<'prepare> {
    pub fn new(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<TextFramePacket>,
        submit_packet: Box<TextSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self {
                resource_context: prepare_context.resource_context.clone(),
                font_atlas_cache: {
                    TrustCell::new(
                        prepare_context
                            .render_resources
                            .fetch_mut::<FontAtlasCache>(),
                    )
                },
                device_context: prepare_context.device_context.clone(),
            },
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for TextPrepareJob<'prepare> {
    fn begin_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        let mut per_frame_submit_data = TextPerFrameSubmitData::default();

        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();
        let dyn_resource_allocator_set = self.resource_context.create_dyn_resource_allocator_set();
        let font_atlas_cache = &mut self.font_atlas_cache.borrow_mut();

        let draw_vertices_result = font_atlas_cache
            .generate_vertices(
                &per_frame_data.text_draw_commands,
                &per_frame_data.font_assets,
                &dyn_resource_allocator_set,
            )
            .unwrap();

        per_frame_submit_data.draw_call_metas = draw_vertices_result.draw_call_metas;
        per_frame_submit_data.image_updates = draw_vertices_result.image_updates;
        per_frame_submit_data
            .draw_call_buffers
            .reserve(draw_vertices_result.draw_call_buffer_data.len());

        if !per_frame_submit_data.draw_call_metas.is_empty() {
            let per_font_descriptor_set_layout = &per_frame_data
                .text_material_pass
                .as_ref()
                .unwrap()
                .get_raw()
                .descriptor_set_layouts[shaders::text_frag::TEX_DESCRIPTOR_SET_INDEX];

            //
            // Create per-font descriptor sets (i.e. a font atlas texture)
            //
            for image in &draw_vertices_result.font_atlas_images {
                let per_font_descriptor_set = descriptor_set_allocator
                    .create_descriptor_set_with_writer(
                        per_font_descriptor_set_layout,
                        shaders::text_frag::DescriptorSet0Args { tex: &image },
                    )
                    .unwrap();

                per_frame_submit_data
                    .per_font_descriptor_sets
                    .push(per_font_descriptor_set);
            }
        }

        //
        // Update the vertex buffers
        //

        for draw_call in draw_vertices_result.draw_call_buffer_data {
            let vertex_buffer = self
                .device_context
                .create_buffer(&RafxBufferDef::for_staging_vertex_buffer_data(
                    &draw_call.vertices,
                ))
                .unwrap();

            vertex_buffer
                .copy_to_host_visible_buffer(draw_call.vertices.as_slice())
                .unwrap();

            let vertex_buffer = dyn_resource_allocator_set.insert_buffer(vertex_buffer);

            let index_buffer = self
                .device_context
                .create_buffer(&RafxBufferDef::for_staging_index_buffer_data(
                    &draw_call.indices,
                ))
                .unwrap();

            index_buffer
                .copy_to_host_visible_buffer(draw_call.indices.as_slice())
                .unwrap();

            let index_buffer = dyn_resource_allocator_set.insert_buffer(index_buffer);

            per_frame_submit_data
                .draw_call_buffers
                .push(TextDrawCallBuffers {
                    vertex_buffer,
                    index_buffer,
                });
        }

        context
            .submit_packet()
            .per_frame_submit_data()
            .set(per_frame_submit_data);
    }

    fn end_per_view_prepare(
        &self,
        context: &PreparePerViewContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        if per_frame_data.text_material_pass.is_none() {
            return;
        }

        let text_material_pass = per_frame_data.text_material_pass.as_ref().unwrap();
        let per_view_descriptor_set_layout = &text_material_pass.get_raw().descriptor_set_layouts
            [shaders::text_vert::PER_VIEW_DATA_DESCRIPTOR_SET_INDEX];

        //
        // Setup the vertex shader descriptor set
        //

        let view = context.view();
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

        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();
        context
            .view_submit_packet()
            .per_view_submit_data()
            .set(TextPerViewSubmitData {
                descriptor_set_arc: descriptor_set_allocator
                    .create_descriptor_set_with_writer(
                        per_view_descriptor_set_layout,
                        shaders::text_vert::DescriptorSet1Args {
                            per_view_data: &text_view,
                        },
                    )
                    .ok(),
            });

        let per_frame_submit_data = context.per_frame_submit_data();

        // Submit a single node for each view
        // TODO: Submit separate nodes for transparency/text positioned in 3d

        for draw_call in per_frame_submit_data.draw_call_metas.iter() {
            context
                .view_submit_packet()
                .push_submit_node::<UiRenderPhase>((), 0, draw_call.z_position);
        }
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = TextRenderFeatureTypes;
    type SubmitPacketDataT = TextRenderFeatureTypes;
}
