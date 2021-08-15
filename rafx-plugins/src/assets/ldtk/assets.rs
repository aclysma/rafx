use crate::phases::OpaqueRenderPhase;
use crate::shaders;
use fnv::FnvHashMap;
use glam::Vec3;
use rafx::api::RafxResult;
use rafx::assets::{
    AssetManager, BufferAsset, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, ImageAsset,
    MaterialInstanceAsset,
};
use rafx::distill::loader::handle::Handle;
use rafx::framework::{BufferResource, DescriptorSetArc, ResourceArc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

pub type LevelUid = i64;
pub type TileSetUid = i64;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LdtkLayerDrawCallData {
    pub vertex_data_offset_in_bytes: u32,
    pub index_data_offset_in_bytes: u32,
    pub index_count: u32,
    pub z_pos: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LdtkLayerData {
    pub material_instance: Handle<MaterialInstanceAsset>,
    pub draw_call_data: Vec<LdtkLayerDrawCallData>,
    pub z_pos: f32,
    pub world_x_pos: i64,
    pub world_y_pos: i64,
    pub grid_width: i64,
    pub grid_height: i64,
    pub grid_size: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LdtkLevelData {
    pub layer_data: Vec<LdtkLayerData>,
    pub vertex_data: Option<Handle<BufferAsset>>,
    pub index_data: Option<Handle<BufferAsset>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LdtkTileSet {
    pub image: Handle<ImageAsset>,
    pub material_instance: Handle<MaterialInstanceAsset>,
    pub image_width: u32,
    pub image_height: u32,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "98c635e3-b277-422f-bd6a-bf0b83814211"]
pub struct LdtkAssetData {
    pub tilesets: FnvHashMap<TileSetUid, LdtkTileSet>,
    pub levels: FnvHashMap<LevelUid, LdtkLevelData>,
}

#[derive(Clone, Debug)]
pub struct LdtkLayer {
    pub per_layer_descriptor_set: DescriptorSetArc,
    pub width: i64,
    pub height: i64,
    pub center: Vec3,
}

#[derive(Clone, Debug)]
pub struct LdtkLevel {
    pub layers: Vec<LdtkLayer>,
    pub vertex_buffer: Option<ResourceArc<BufferResource>>,
    pub index_buffer: Option<ResourceArc<BufferResource>>,
}

#[derive(Debug)]
pub struct LdtkProjectAssetInner {
    pub data: LdtkAssetData,
    pub levels: FnvHashMap<LevelUid, LdtkLevel>,
}

#[derive(TypeUuid, Clone, Debug)]
#[uuid = "231e4fd5-add2-4024-a479-d8181b3e52a3"]
pub struct LdtkProjectAsset {
    pub inner: Arc<LdtkProjectAssetInner>,
}

pub struct LdtkLoadHandler;

impl DefaultAssetTypeLoadHandler<LdtkAssetData, LdtkProjectAsset> for LdtkLoadHandler {
    #[profiling::function]
    fn load(
        asset_manager: &mut AssetManager,
        ldtk_asset: LdtkAssetData,
    ) -> RafxResult<LdtkProjectAsset> {
        let mut levels = FnvHashMap::<LevelUid, LdtkLevel>::default();
        for (&level_uid, level_data) in &ldtk_asset.levels {
            let mut layers = Vec::default();
            for layer_data in &level_data.layer_data {
                let material_instance = asset_manager
                    .latest_asset(&layer_data.material_instance)
                    .unwrap();
                let opaque_phase_pass_index = material_instance
                    .material
                    .find_pass_by_phase::<OpaqueRenderPhase>()
                    .expect("tileset material must have pass for opaque phase");
                let tileset_image_set_index = shaders::tile_layer_frag::TEX_DESCRIPTOR_SET_INDEX;
                let descriptor_set = material_instance.material_descriptor_sets
                    [opaque_phase_pass_index][tileset_image_set_index]
                    .clone()
                    .unwrap();

                let width = layer_data.grid_width * layer_data.grid_size;
                let height = layer_data.grid_height * layer_data.grid_size;
                layers.push(LdtkLayer {
                    per_layer_descriptor_set: descriptor_set,
                    center: Vec3::new(
                        (layer_data.world_x_pos + (width / 2)) as f32,
                        (layer_data.world_y_pos + (height / 2)) as f32,
                        layer_data.z_pos,
                    ),
                    width,
                    height,
                });
            }

            let mut vertex_buffer = None;
            let mut index_buffer = None;

            if let (Some(vertex_data_handle), Some(index_data_handle)) =
                (&level_data.vertex_data, &level_data.index_data)
            {
                vertex_buffer = Some(
                    asset_manager
                        .latest_asset(&vertex_data_handle)
                        .unwrap()
                        .buffer
                        .clone(),
                );
                index_buffer = Some(
                    asset_manager
                        .latest_asset(&index_data_handle)
                        .unwrap()
                        .buffer
                        .clone(),
                );
            }

            levels.insert(
                level_uid,
                LdtkLevel {
                    layers,
                    vertex_buffer,
                    index_buffer,
                },
            );
        }

        let inner = LdtkProjectAssetInner {
            data: ldtk_asset,
            levels,
        };

        Ok(LdtkProjectAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type LdtkAssetType = DefaultAssetTypeHandler<LdtkAssetData, LdtkProjectAsset, LdtkLoadHandler>;
