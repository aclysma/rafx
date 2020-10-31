use renderer::nodes::{
    RenderView, ViewSubmitNodes, FeatureSubmitNodes, FeatureCommandWriter, RenderFeatureIndex,
    FramePacket, RenderFeature, PrepareJob,
};
use crate::features::debug3d::{
    Debug3dRenderFeature, ExtractedDebug3dData, Debug3dDrawCall, Debug3dVertex,
    Debug3dUniformBufferObject,
};
use crate::phases::OpaqueRenderPhase;
use super::write::Debug3dCommandWriter;
use crate::render_contexts::{RenderJobWriteContext, RenderJobPrepareContext};
use renderer::vulkan::VkBuffer;
use ash::vk;
use renderer::assets::resources::{ResourceArc, MaterialPassResource};

pub struct Debug3dPrepareJobImpl {
    debug3d_material_pass: ResourceArc<MaterialPassResource>,
    extracted_debug3d_data: ExtractedDebug3dData,
}

impl Debug3dPrepareJobImpl {
    pub(super) fn new(
        debug3d_material_pass: ResourceArc<MaterialPassResource>,
        extracted_debug3d_data: ExtractedDebug3dData,
    ) -> Self {
        Debug3dPrepareJobImpl {
            debug3d_material_pass,
            extracted_debug3d_data,
        }
    }
}

impl PrepareJob<RenderJobPrepareContext, RenderJobWriteContext> for Debug3dPrepareJobImpl {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        _frame_packet: &FramePacket,
        views: &[&RenderView],
    ) -> (
        Box<dyn FeatureCommandWriter<RenderJobWriteContext>>,
        FeatureSubmitNodes,
    ) {
        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();
        let per_view_descriptor_set_layout = &self
            .debug3d_material_pass
            .get_raw()
            .pipeline_layout
            .get_raw()
            .descriptor_sets[0];

        let mut per_view_descriptor_sets = Vec::default();
        for view in views {
            let debug3d_view = Debug3dUniformBufferObject {
                view_proj: (view.projection_matrix() * view.view_matrix()).to_cols_array_2d(),
            };

            let mut descriptor_set = descriptor_set_allocator
                .create_dyn_descriptor_set_uninitialized(per_view_descriptor_set_layout)
                .unwrap();
            descriptor_set.set_buffer_data(0, &debug3d_view);
            descriptor_set.flush(&mut descriptor_set_allocator).unwrap();

            per_view_descriptor_sets.resize(per_view_descriptor_sets.len().max(view.view_index() as usize + 1), None);
            per_view_descriptor_sets[view.view_index() as usize] = Some(descriptor_set.descriptor_set().clone());
        }

        // let per_view_descriptor_sets: Vec<_> = views
        //     .iter()
        //     .map(|view| {
        //         let debug3d_view = Debug3dUniformBufferObject {
        //             view_proj: (view.projection_matrix() * view.view_matrix()).to_cols_array_2d(),
        //         };
        //
        //         let mut descriptor_set = descriptor_set_allocator
        //             .create_dyn_descriptor_set_uninitialized(per_view_descriptor_set_layout)
        //             .unwrap();
        //         descriptor_set.set_buffer_data(0, &debug3d_view);
        //         descriptor_set.flush(&mut descriptor_set_allocator).unwrap();
        //         descriptor_set.descriptor_set().clone()
        //     })
        //     .collect();

        //
        // Gather the raw draw data
        //
        let line_lists = &self.extracted_debug3d_data.line_lists;
        let mut draw_calls = Vec::with_capacity(line_lists.len());
        let dyn_resource_allocator = prepare_context
            .resource_context
            .create_dyn_resource_allocator_set();

        let mut vertex_list: Vec<Debug3dVertex> = vec![];
        for line_list in line_lists {
            let vertex_buffer_first_element = vertex_list.len() as u32;

            for vertex_pos in &line_list.points {
                vertex_list.push(Debug3dVertex {
                    pos: (*vertex_pos).into(),
                    color: line_list.color.into(),
                });
            }

            let draw_call = Debug3dDrawCall {
                first_element: vertex_buffer_first_element,
                count: line_list.points.len() as u32,
            };

            draw_calls.push(draw_call);
        }

        // We would probably want to support multiple buffers at some point
        let vertex_buffer = if !draw_calls.is_empty() {
            let vertex_buffer_size =
                vertex_list.len() as u64 * std::mem::size_of::<Debug3dVertex>() as u64;
            let mut vertex_buffer = VkBuffer::new(
                &prepare_context.device_context,
                vk_mem::MemoryUsage::CpuToGpu,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                vertex_buffer_size,
            )
            .unwrap();

            vertex_buffer
                .write_to_host_visible_buffer(vertex_list.as_slice())
                .unwrap();

            Some(dyn_resource_allocator.insert_buffer(vertex_buffer))
        } else {
            None
        };

        //
        // Submit a single node for each view
        // TODO: Submit separate nodes for transparency
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for view in views {
            let mut view_submit_nodes =
                ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());
            view_submit_nodes.add_submit_node::<OpaqueRenderPhase>(0, 0, 0.0);
            submit_nodes.add_submit_nodes_for_view(view, view_submit_nodes);
        }

        let writer = Box::new(Debug3dCommandWriter {
            draw_calls,
            vertex_buffer,
            debug3d_material_pass: self.debug3d_material_pass,
            per_view_descriptor_sets,
        });

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        Debug3dRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        Debug3dRenderFeature::feature_index()
    }
}
