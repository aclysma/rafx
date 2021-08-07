use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::UiRenderPhase;
use rafx::api::{RafxBufferDef, RafxDeviceContext, RafxMemoryUsage, RafxResourceType};
use rafx::framework::{ImageViewResource, ResourceArc, ResourceContext};

pub struct ImGuiPrepareJob {
    resource_context: ResourceContext,
    device_context: RafxDeviceContext,
    font_atlas: ResourceArc<ImageViewResource>,
}

impl ImGuiPrepareJob {
    pub fn new<'prepare>(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<ImGuiFramePacket>,
        submit_packet: Box<ImGuiSubmitPacket>,
        font_atlas: ResourceArc<ImageViewResource>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self {
                resource_context: prepare_context.resource_context.clone(),
                device_context: prepare_context.device_context.clone(),
                font_atlas,
            },
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for ImGuiPrepareJob {
    fn begin_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        let per_frame_data = context.per_frame_data();
        let mut per_frame_submit_data = ImGuiPerFrameSubmitData::default();

        let descriptor_set_layouts = &per_frame_data
            .imgui_material_pass
            .as_ref()
            .unwrap()
            .get_raw()
            .descriptor_set_layouts;

        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();
        let dyn_resource_allocator_set = self.resource_context.create_dyn_resource_allocator_set();

        per_frame_submit_data.per_view_descriptor_set = descriptor_set_allocator
            .create_descriptor_set_with_writer(
                &descriptor_set_layouts[shaders::imgui_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX],
                shaders::imgui_vert::DescriptorSet0Args {
                    uniform_buffer: &per_frame_data.view_ubo,
                },
            )
            .ok();

        per_frame_submit_data.per_font_descriptor_set = descriptor_set_allocator
            .create_descriptor_set_with_writer(
                &descriptor_set_layouts[shaders::imgui_frag::TEX_DESCRIPTOR_SET_INDEX],
                shaders::imgui_frag::DescriptorSet1Args {
                    tex: &self.font_atlas,
                },
            )
            .ok();

        if let Some(draw_data) = &per_frame_data.imgui_draw_data {
            for draw_list in draw_data.draw_lists() {
                let vertex_buffer_size = draw_list.vertex_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawVert>() as u64;

                let vertex_buffer = self
                    .device_context
                    .create_buffer(&RafxBufferDef {
                        size: vertex_buffer_size,
                        memory_usage: RafxMemoryUsage::CpuToGpu,
                        resource_type: RafxResourceType::VERTEX_BUFFER,
                        ..Default::default()
                    })
                    .unwrap();

                vertex_buffer
                    .copy_to_host_visible_buffer(draw_list.vertex_buffer())
                    .unwrap();

                let vertex_buffer = dyn_resource_allocator_set.insert_buffer(vertex_buffer);
                per_frame_submit_data.vertex_buffers.push(vertex_buffer);

                let index_buffer_size = draw_list.index_buffer().len() as u64
                    * std::mem::size_of::<imgui::DrawIdx>() as u64;

                let index_buffer = self
                    .device_context
                    .create_buffer(&RafxBufferDef {
                        size: index_buffer_size,
                        memory_usage: RafxMemoryUsage::CpuToGpu,
                        resource_type: RafxResourceType::INDEX_BUFFER,
                        ..Default::default()
                    })
                    .unwrap();

                index_buffer
                    .copy_to_host_visible_buffer(draw_list.index_buffer())
                    .unwrap();

                let index_buffer = dyn_resource_allocator_set.insert_buffer(index_buffer);
                per_frame_submit_data.index_buffers.push(index_buffer);
            }
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
        if per_frame_data.imgui_draw_data.is_none() || per_frame_data.imgui_material_pass.is_none()
        {
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

    type FramePacketDataT = ImGuiRenderFeatureTypes;
    type SubmitPacketDataT = ImGuiRenderFeatureTypes;
}
