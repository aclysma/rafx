rafx::declare_render_feature_prepare_job!();

use super::write::Debug3DVertex;
use crate::features::debug3d::public::debug3d_resource::LineList3D;
use crate::phases::OpaqueRenderPhase;
use rafx::api::{RafxBufferDef, RafxMemoryUsage, RafxResourceType};
use rafx::framework::{MaterialPassResource, ResourceArc};

pub type Debug3dUniformBufferObject = shaders::debug_vert::PerFrameUboUniform;

pub struct PrepareJobImpl {
    debug3d_material_pass: ResourceArc<MaterialPassResource>,
    line_lists: Vec<LineList3D>,
}

impl PrepareJobImpl {
    pub(super) fn new(
        debug3d_material_pass: ResourceArc<MaterialPassResource>,
        line_lists: Vec<LineList3D>,
    ) -> Self {
        PrepareJobImpl {
            debug3d_material_pass,
            line_lists,
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
        profiling::scope!(prepare_scope);

        let mut writer = Box::new(FeatureCommandWriterImpl::new(
            self.debug3d_material_pass.clone(),
            self.line_lists.len(),
        ));

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        let per_view_descriptor_set_layout =
            &self.debug3d_material_pass.get_raw().descriptor_set_layouts
                [shaders::debug_vert::PER_FRAME_DATA_DESCRIPTOR_SET_INDEX];

        for view in views {
            let debug3d_view = Debug3dUniformBufferObject {
                view_proj: (view.projection_matrix() * view.view_matrix()).to_cols_array_2d(),
            };

            let descriptor_set = descriptor_set_allocator
                .create_descriptor_set(
                    per_view_descriptor_set_layout,
                    shaders::debug_vert::DescriptorSet0Args {
                        per_frame_data: &debug3d_view,
                    },
                )
                .unwrap();

            writer.push_per_view_descriptor_set(view.view_index(), descriptor_set);
        }

        //
        // Gather the raw draw data
        //
        let dyn_resource_allocator = prepare_context
            .resource_context
            .create_dyn_resource_allocator_set();

        let mut vertex_list: Vec<Debug3DVertex> = vec![];
        for line_list in &self.line_lists {
            let vertex_buffer_first_element = vertex_list.len() as u32;

            for vertex_pos in &line_list.points {
                vertex_list.push(Debug3DVertex {
                    pos: (*vertex_pos).into(),
                    color: line_list.color.into(),
                });
            }

            writer.push_draw_call(vertex_buffer_first_element, line_list.points.len());
        }

        // We would probably want to support multiple buffers at some point

        let vertex_buffer = if !writer.draw_calls().is_empty() {
            let vertex_buffer_size =
                vertex_list.len() as u64 * std::mem::size_of::<Debug3DVertex>() as u64;

            let vertex_buffer = prepare_context
                .device_context
                .create_buffer(&RafxBufferDef {
                    size: vertex_buffer_size,
                    memory_usage: RafxMemoryUsage::CpuToGpu,
                    resource_type: RafxResourceType::VERTEX_BUFFER,
                    ..Default::default()
                })
                .unwrap();

            vertex_buffer
                .copy_to_host_visible_buffer(vertex_list.as_slice())
                .unwrap();

            Some(dyn_resource_allocator.insert_buffer(vertex_buffer))
        } else {
            None
        };

        writer.set_vertex_buffer(vertex_buffer);

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

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        render_feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }
}
