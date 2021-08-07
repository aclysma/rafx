use rafx::render_feature_prepare_job_predule::*;

use super::*;
use crate::phases::WireframeRenderPhase;
use rafx::api::{RafxBufferDef, RafxDeviceContext, RafxMemoryUsage, RafxResourceType};
use rafx::framework::ResourceContext;

pub struct Debug3DPrepareJob {
    resource_context: ResourceContext,
    device_context: RafxDeviceContext,
}

impl Debug3DPrepareJob {
    pub fn new<'prepare>(
        prepare_context: &RenderJobPrepareContext<'prepare>,
        frame_packet: Box<Debug3DFramePacket>,
        submit_packet: Box<Debug3DSubmitPacket>,
    ) -> Arc<dyn RenderFeaturePrepareJob<'prepare> + 'prepare> {
        Arc::new(PrepareJob::new(
            Self {
                resource_context: prepare_context.resource_context.clone(),
                device_context: prepare_context.device_context.clone(),
            },
            frame_packet,
            submit_packet,
        ))
    }
}

impl<'prepare> PrepareJobEntryPoints<'prepare> for Debug3DPrepareJob {
    fn begin_per_frame_prepare(
        &self,
        context: &PreparePerFrameContext<'prepare, '_, Self>,
    ) {
        let mut per_frame_submit_data = Debug3DPerFrameSubmitData::default();

        //
        // Gather the raw draw data
        //
        let line_lists = &context.per_frame_data().line_lists;
        for line_list in line_lists.iter() {
            let vertex_buffer_first_element = per_frame_submit_data.vertex_list.len() as u32;

            for vertex_pos in &line_list.points {
                per_frame_submit_data.vertex_list.push(Debug3DVertex {
                    pos: (*vertex_pos).into(),
                    color: line_list.color.into(),
                });
            }

            per_frame_submit_data.draw_calls.push(Debug3DDrawCall {
                first_element: vertex_buffer_first_element,
                count: line_list.points.len() as u32,
            });
        }

        // We would probably want to support multiple buffers at some point

        let dyn_resource_allocator_set = self.resource_context.create_dyn_resource_allocator_set();
        per_frame_submit_data.vertex_buffer = if !per_frame_submit_data.draw_calls.is_empty() {
            let vertex_buffer_size = per_frame_submit_data.vertex_list.len() as u64
                * std::mem::size_of::<Debug3DVertex>() as u64;

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
                .copy_to_host_visible_buffer(per_frame_submit_data.vertex_list.as_slice())
                .unwrap();

            Some(dyn_resource_allocator_set.insert_buffer(vertex_buffer))
        } else {
            None
        };

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
        if per_frame_data.debug3d_material_pass.is_none() {
            return;
        }

        let debug3d_material_pass = per_frame_data.debug3d_material_pass.as_ref().unwrap();
        let per_view_descriptor_set_layout = &debug3d_material_pass
            .get_raw()
            .descriptor_set_layouts[shaders::debug_vert::PER_FRAME_DATA_DESCRIPTOR_SET_INDEX];

        let view = context.view();
        let debug3d_view = Debug3DUniformBufferObject {
            view_proj: (view.projection_matrix() * view.view_matrix()).to_cols_array_2d(),
        };

        let mut descriptor_set_allocator = self.resource_context.create_descriptor_set_allocator();
        context
            .view_submit_packet()
            .per_view_submit_data()
            .set(Debug3DPerViewSubmitData {
                descriptor_set_arc: descriptor_set_allocator
                    .create_descriptor_set_with_writer(
                        per_view_descriptor_set_layout,
                        shaders::debug_vert::DescriptorSet0Args {
                            per_frame_data: &debug3d_view,
                        },
                    )
                    .ok(),
            });

        context
            .view_submit_packet()
            .push_submit_node::<WireframeRenderPhase>((), 0, 0.);
    }

    fn feature_debug_constants(&self) -> &'static RenderFeatureDebugConstants {
        super::render_feature_debug_constants()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        super::render_feature_index()
    }

    type RenderObjectInstanceJobContextT = DefaultJobContext;
    type RenderObjectInstancePerViewJobContextT = DefaultJobContext;

    type FramePacketDataT = Debug3DRenderFeatureTypes;
    type SubmitPacketDataT = Debug3DRenderFeatureTypes;
}
