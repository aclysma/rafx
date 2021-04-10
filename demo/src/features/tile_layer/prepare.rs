use rafx::render_feature_prepare_job_predule::*;

use super::{RenderFeatureType, TileLayerRenderNode, TileLayerWriteJob};
use crate::phases::TransparentRenderPhase;
use rafx::framework::{MaterialPassResource, ResourceArc};

/// Per-pass "global" data
pub type TileLayerUniformBufferObject = shaders::tile_layer_vert::ArgsUniform;

pub struct TileLayerPrepareJob {
    visible_render_nodes: Vec<TileLayerRenderNode>,
    tile_layer_material: ResourceArc<MaterialPassResource>,
}

impl TileLayerPrepareJob {
    pub(super) fn new(
        visible_render_nodes: Vec<TileLayerRenderNode>,
        tile_layer_material: ResourceArc<MaterialPassResource>,
    ) -> Self {
        TileLayerPrepareJob {
            visible_render_nodes,
            tile_layer_material,
        }
    }
}

impl PrepareJob for TileLayerPrepareJob {
    fn prepare(
        self: Box<Self>,
        prepare_context: &RenderJobPrepareContext,
        frame_packet: &FramePacket,
        views: &[RenderView],
    ) -> (Box<dyn WriteJob>, FeatureSubmitNodes) {
        profiling::scope!(super::prepare_scope);

        let mut descriptor_set_allocator = prepare_context
            .resource_context
            .create_descriptor_set_allocator();

        let mut writer = Box::new(TileLayerWriteJob::new(
            self.tile_layer_material.clone(),
            self.visible_render_nodes,
        ));

        //
        // Add submit nodes per view
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for view in views {
            if let Some(view_nodes) = frame_packet.view_nodes(view, super::render_feature_index()) {
                let visible_render_nodes = writer.visible_render_nodes();

                let mut view_submit_nodes =
                    ViewSubmitNodes::new(super::render_feature_index(), view.render_phase_mask());

                for view_node in view_nodes {
                    let frame_node_index = view_node.frame_node_index();
                    let layer_z_position =
                        visible_render_nodes[frame_node_index as usize].z_position;
                    let distance = (layer_z_position - view.eye_position().z).abs();
                    view_submit_nodes.add_submit_node::<TransparentRenderPhase>(
                        frame_node_index,
                        0,
                        distance,
                    );
                }

                submit_nodes.add_submit_nodes_for_view(&view, view_submit_nodes);
            }

            if view.is_relevant::<TransparentRenderPhase, RenderFeatureType>() {
                let layout = &self.tile_layer_material.get_raw().descriptor_set_layouts
                    [shaders::tile_layer_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX];
                let descriptor_set = descriptor_set_allocator
                    .create_descriptor_set(
                        &*layout,
                        shaders::tile_layer_vert::DescriptorSet0Args {
                            uniform_buffer: &shaders::tile_layer_vert::ArgsUniform {
                                mvp: view.view_proj().to_cols_array_2d(),
                            },
                        },
                    )
                    .unwrap();

                writer.push_per_view_descriptor_set(view.view_index(), descriptor_set);
            }
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
