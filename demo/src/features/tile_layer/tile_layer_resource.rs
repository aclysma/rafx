use std::sync::Arc;
use crate::assets::ldtk::LdtkProjectAsset;
use rafx::distill::loader::handle::Handle;
use rafx::assets::AssetManager;
use fnv::FnvHashMap;
use crate::features::tile_layer::{TileLayerRenderNodeHandle, TileLayerRenderNodeSet, TileLayerRenderNode};
use rafx::visibility::{StaticVisibilityNodeSet, StaticAabbVisibilityNode, StaticAabbVisibilityNodeHandle};

#[derive(Default)]
pub struct TileLayerResource {
    project: Option<Handle<LdtkProjectAsset>>,
    render_nodes: Vec<TileLayerRenderNodeHandle>,
    visibiility_nodes: Vec<StaticAabbVisibilityNodeHandle>,
    //layer_count: u32,
}

impl TileLayerResource {
    pub fn project(&self) -> &Option<Handle<LdtkProjectAsset>> {
        &self.project
    }

    pub fn render_nodes(&self) -> &Vec<TileLayerRenderNodeHandle> {
        &self.render_nodes
    }

    pub fn set_project(
        &mut self,
        project: &Handle<LdtkProjectAsset>,
        asset_manager: &AssetManager,
        tile_layer_render_nodes: &mut TileLayerRenderNodeSet,
        static_visibility: &mut StaticVisibilityNodeSet,
    ) {
        self.project = Some(project.clone());
        //self.layer_count = Self::total_layer_count(&project, asset_manager);

        let project_asset = asset_manager.committed_asset(project).unwrap();

        for (level_uid, level) in &project_asset.inner.levels {
            for (layer_index, layer) in level.layers.iter().enumerate() {
                if let (Some(vertex_buffer), Some(index_buffer)) = (&level.vertex_buffer, &level.index_buffer) {
                    let layer_data = &project_asset.inner.data.levels[level_uid].layer_data[layer_index];
                    let render_node = tile_layer_render_nodes.register_tile_layer(TileLayerRenderNode {
                        per_layer_descriptor_set: layer.per_layer_descriptor_set.clone(),
                        draw_call_data: layer_data.draw_call_data.clone(),
                        vertex_buffer: vertex_buffer.clone(),
                        index_buffer: index_buffer.clone(),
                    });

                    let visibility_node = static_visibility.register_static_aabb(StaticAabbVisibilityNode {
                        handle: render_node.as_raw_generic_handle()
                    });

                    self.render_nodes.push(render_node);
                    self.visibiility_nodes.push(visibility_node);
                }
            }
        }
    }

    pub fn clear_project(&mut self) {
        self.project = None;
        self.render_nodes.clear();
        //self.total_layer_count = 0;
    }

    // // Need this to reserve appropriate number of frame nodes, one per each layer/level/assets
    // pub fn total_layer_count(project: &Handle<LdtkProjectAsset>, asset_manager: &AssetManager) -> u32 {
    //     let mut total_layer_count = 0;
    //     let asset = asset_manager.committed_asset(&project);
    //
    //     if let Some(asset) = asset {
    //         for (level_uid, level) in &asset.inner.levels {
    //             total_layer_count += level.layers.len();
    //         }
    //     }
    //
    //     total_layer_count
    // }
}
