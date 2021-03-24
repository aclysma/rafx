use super::TileLayerCommandWriter;
use crate::features::tile_layer::{TileLayerRenderFeature, TileLayerVertex, TileLayerRenderNode};
use crate::phases::OpaqueRenderPhase;
use crate::phases::TransparentRenderPhase;
use fnv::FnvHashMap;
use glam::Vec3;
use rafx::api::{RafxBufferDef, RafxMemoryUsage, RafxResourceType};
use rafx::framework::{DescriptorSetArc, ImageViewResource, MaterialPassResource, ResourceArc};
use rafx::nodes::{
    FeatureCommandWriter, FeatureSubmitNodes, FramePacket, PrepareJob, RenderFeature,
    RenderFeatureIndex, RenderJobPrepareContext, RenderView, ViewSubmitNodes,
};
use crate::assets::ldtk::LdtkProjectAsset;

pub struct TileLayerPrepareJob {
    //ldtk_assets: Vec<LdtkProjectAsset>,
    visible_render_nodes: Vec<TileLayerRenderNode>,
    tile_layer_material: ResourceArc<MaterialPassResource>,
}

impl TileLayerPrepareJob {
    pub(super) fn new(
        visible_render_nodes: Vec<TileLayerRenderNode>,
        //ldtk_assets: Vec<LdtkProjectAsset>,
        tile_layer_material: ResourceArc<MaterialPassResource>,
    ) -> Self {
        TileLayerPrepareJob {
            visible_render_nodes,
            //ldtk_assets: extracted_tile_layer_data,
            //ldtk_assets,
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
    ) -> (Box<dyn FeatureCommandWriter>, FeatureSubmitNodes) {
        profiling::scope!("TileLayer Prepare");

        let mut descriptor_set_allocator = prepare_context.resource_context.create_descriptor_set_allocator();
        let mut per_view_descriptor_sets = Vec::default();

        // let extents_width = 900;
        // let extents_height = 600;
        // let aspect_ratio = extents_width as f32 / extents_height as f32;
        // let half_width = 450.0;
        // let half_height = 450.0 / aspect_ratio;
        let view_proj = glam::Mat4::orthographic_rh(
            0.0,
            900.0,
            256.0 - 600.0,
            256.0,
            1000.0,
            -1000.0,
        );

        //
        // Add submit nodes per view
        //
        let mut submit_nodes = FeatureSubmitNodes::default();
        for view in views {

            if let Some(view_nodes) = frame_packet.view_nodes(view, self.feature_index()) {
                let mut view_submit_nodes =
                    ViewSubmitNodes::new(self.feature_index(), view.render_phase_mask());
                for view_node in view_nodes {
                    let frame_node_index = view_node.frame_node_index();

                    //let render_node = &self.visible_render_nodes[frame_node_index as usize];

                    //if let Some(extracted_data) =
                    //    &self.visible_render_nodes[frame_node_index as usize]

                    let layer_z_position = 0.0;
                    //{
                        let distance = (layer_z_position - view.eye_position().z()).abs();

                        view_submit_nodes.add_submit_node::<TransparentRenderPhase>(
                            frame_node_index,
                            0,
                            distance,
                        );
                    //}
                }

                submit_nodes.add_submit_nodes_for_view(&view, view_submit_nodes);
            }
        //
        //     //TODO: Multi-view support for tile_layers. Not clear on if we want to do a screen-space view specifically
        //     // for tile_layers
        //     //TODO: Extents is hard-coded
            let layout = &self.tile_layer_material.get_raw().descriptor_set_layouts
                [shaders::tile_layer_vert::UNIFORM_BUFFER_DESCRIPTOR_SET_INDEX];
            let descriptor_set = descriptor_set_allocator
                .create_descriptor_set(
                    &*layout,
                    shaders::tile_layer_vert::DescriptorSet0Args {
                        uniform_buffer: &shaders::tile_layer_vert::ArgsUniform {
                            mvp: view_proj.to_cols_array_2d(),
                        },
                    },
                )
                .unwrap();

            per_view_descriptor_sets.resize(
                per_view_descriptor_sets
                    .len()
                    .max(view.view_index() as usize + 1),
                None,
            );
            per_view_descriptor_sets[view.view_index() as usize] = Some(descriptor_set);
        }

        let writer = Box::new(TileLayerCommandWriter {
            per_view_descriptor_sets,
            visible_render_nodes: self.visible_render_nodes,
            tile_layer_material: self.tile_layer_material,
            // draw_calls,
            // vertex_buffers,
            // index_buffers,
            // per_view_descriptor_sets,
            // tile_layer_material: self.tile_layer_material,
        });

        (writer, submit_nodes)
    }

    fn feature_debug_name(&self) -> &'static str {
        TileLayerRenderFeature::feature_debug_name()
    }

    fn feature_index(&self) -> RenderFeatureIndex {
        TileLayerRenderFeature::feature_index()
    }
}
