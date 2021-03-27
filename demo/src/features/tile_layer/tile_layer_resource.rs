use crate::assets::ldtk::LdtkProjectAsset;
use crate::features::tile_layer::{
    TileLayerRenderNode, TileLayerRenderNodeHandle, TileLayerRenderNodeSet,
};
use rafx::assets::AssetManager;
use rafx::distill::loader::handle::Handle;
use rafx::visibility::{
    StaticAabbVisibilityNode, StaticAabbVisibilityNodeHandle, StaticVisibilityNodeSet,
};

#[derive(Default)]
pub struct TileLayerResource {
    project: Option<Handle<LdtkProjectAsset>>,
    render_nodes: Vec<TileLayerRenderNodeHandle>,
    visibiility_nodes: Vec<StaticAabbVisibilityNodeHandle>,
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
        self.clear_project();
        self.project = Some(project.clone());

        let project_asset = asset_manager.committed_asset(project).unwrap();

        for (level_uid, level) in &project_asset.inner.levels {
            for (layer_index, layer) in level.layers.iter().enumerate() {
                if let (Some(vertex_buffer), Some(index_buffer)) =
                    (&level.vertex_buffer, &level.index_buffer)
                {
                    let layer_data =
                        &project_asset.inner.data.levels[level_uid].layer_data[layer_index];
                    let render_node =
                        tile_layer_render_nodes.register_tile_layer(TileLayerRenderNode {
                            per_layer_descriptor_set: layer.per_layer_descriptor_set.clone(),
                            draw_call_data: layer_data.draw_call_data.clone(),
                            vertex_buffer: vertex_buffer.clone(),
                            index_buffer: index_buffer.clone(),
                            z_position: layer_data.z_pos,
                        });

                    let visibility_node =
                        static_visibility.register_static_aabb(StaticAabbVisibilityNode {
                            handle: render_node.as_raw_generic_handle(),
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
        self.visibiility_nodes.clear();
    }
}
