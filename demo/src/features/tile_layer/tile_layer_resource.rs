use super::{TileLayerRenderNode, TileLayerRenderNodeHandle, TileLayerRenderNodeSet};
use crate::assets::ldtk::LdtkProjectAsset;
use glam::{Quat, Vec3};
use rafx::assets::AssetManager;
use rafx::distill::loader::handle::Handle;
use rafx::visibility::{CullModel, EntityId, VisibilityObjectArc, VisibilityRegion};

#[derive(Default)]
pub struct TileLayerResource {
    project: Option<Handle<LdtkProjectAsset>>,
    render_nodes: Vec<TileLayerRenderNodeHandle>,
    visibility_handles: Vec<VisibilityObjectArc>,
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
        visibility_region: &VisibilityRegion,
    ) {
        self.clear_project();
        self.project = Some(project.clone());

        let project_asset = asset_manager.committed_asset(project).unwrap();

        let mut tile_layer_entity_id: u64 = 0;
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

                    // NOTE(dvd): Not an actual entity, but necessary for the frame packet.
                    tile_layer_entity_id += 1;
                    let handle = visibility_region.register_static_object(
                        EntityId::from(tile_layer_entity_id),
                        CullModel::quad(layer.width as f32, layer.height as f32),
                    );
                    let mut translation = layer.center;
                    translation.y = -translation.y; // NOTE(dvd): +y is up in our world, but _down_ in LDtk.
                    handle.set_transform(translation, Quat::IDENTITY, Vec3::ONE);
                    handle.add_feature(render_node.as_raw_generic_handle());

                    self.visibility_handles.push(handle);
                    self.render_nodes.push(render_node);
                }
            }
        }
    }

    pub fn clear_project(&mut self) {
        self.project = None;
        self.render_nodes.clear();
        self.visibility_handles.clear();
    }
}
