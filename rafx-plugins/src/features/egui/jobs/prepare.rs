use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::UiRenderPhase;
use crate::shaders;
use rafx::api::{RafxBufferDef, RafxDeviceContext};
use rafx::base::resource_map::WriteBorrow;
use rafx::framework::ResourceContext;

pub struct EguiPrepareJob<'prepare> {
    resource_context: ResourceContext,
    device_context: RafxDeviceContext,
    font_atlas_cache: TrustCell<WriteBorrow<'prepare, EguiFontAtlasCache>>,
}

impl<'prepare> EguiPrepareJob<'prepare> {
    pub fn new(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<EguiFramePacket>,
        submit_packet: Box<EguiSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self {
                resource_context: prepare_context.resource_context.clone(),
                device_context: prepare_context.device_context.clone(),
                font_atlas_cache: TrustCell::new(
                    prepare_context
                        .render_resources
                        .fetch_mut::<EguiFontAtlasCache>(),
                ),
            },
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for EguiPrepareJob<'prepare> {
    fn begin_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        let mut per_frame_submit_data = EguiPerFrameSubmitData::default();

        let dyn_resource_allocator_set = self.resource_context.create_dyn_resource_allocator_set();

        let descriptor_set_layouts = &per_frame_data
            .egui_material_pass
            .as_ref()
            .unwrap()
            .get_raw()
            .descriptor_set_layouts;

        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();

        let mut font_atlas_cache = self.font_atlas_cache.borrow_mut();

        per_frame_submit_data.per_view_descriptor_set = descriptor_set_allocator
            .create_descriptor_set_with_writer(
                &descriptor_set_layouts[shaders::egui_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX],
                shaders::egui_vert::DescriptorSet0Args {
                    uniform_buffer: &per_frame_data.view_ubo,
                },
            )
            .ok();

        if let Some(draw_data) = &per_frame_data.egui_draw_data {
            let vertex_buffer = self
                .device_context
                .create_buffer(&RafxBufferDef::for_staging_vertex_buffer_data(
                    &draw_data.vertices,
                ))
                .unwrap();
            vertex_buffer
                .copy_to_host_visible_buffer(&draw_data.vertices)
                .unwrap();
            let vertex_buffer = dyn_resource_allocator_set.insert_buffer(vertex_buffer);
            per_frame_submit_data.vertex_buffer = Some(vertex_buffer);

            let index_buffer = self
                .device_context
                .create_buffer(&RafxBufferDef::for_staging_index_buffer_data(
                    &draw_data.indices,
                ))
                .unwrap();
            index_buffer
                .copy_to_host_visible_buffer(&draw_data.indices)
                .unwrap();
            let index_buffer = dyn_resource_allocator_set.insert_buffer(index_buffer);
            per_frame_submit_data.index_buffer = Some(index_buffer);

            per_frame_submit_data.image_update = font_atlas_cache
                .update(&dyn_resource_allocator_set, &draw_data.font_atlas)
                .unwrap();
        }

        if let Some(font_atlas_resource) = font_atlas_cache.font_atlas_resource().as_ref() {
            per_frame_submit_data.per_font_descriptor_set = descriptor_set_allocator
                .create_descriptor_set_with_writer(
                    &descriptor_set_layouts[shaders::egui_frag::TEX_DESCRIPTOR_SET_INDEX],
                    shaders::egui_frag::DescriptorSet1Args {
                        tex: font_atlas_resource,
                    },
                )
                .ok();
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
        if per_frame_data.egui_draw_data.is_none() || per_frame_data.egui_material_pass.is_none() {
            return;
        }

        //
        // Submit a single node for each view
        //
        context
            .view_submit_packet()
            .push_submit_node::<UiRenderPhase>((), 0, 0.);
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = EguiRenderFeatureTypes;
    type SubmitPacketDataT = EguiRenderFeatureTypes;
}
