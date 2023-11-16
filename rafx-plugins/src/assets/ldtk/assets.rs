use crate::shaders::tile_layer::tile_layer_frag;
use fnv::FnvHashMap;
use glam::Vec3;
use hydrate_base::handle::Handle;
use hydrate_base::LoadHandle;
use hydrate_base::{ArtifactId, AssetId};
use rafx::api::RafxResult;
use rafx::assets::{
    AssetManager, BufferAsset, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler, ImageAsset,
    MaterialInstanceAsset,
};
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HydrateLdtkLayerDataTemp {
    pub material_instance: ArtifactId,
    pub draw_call_data: Vec<LdtkLayerDrawCallData>,
    pub z_pos: f32,
    pub world_x_pos: i64,
    pub world_y_pos: i64,
    pub grid_width: i64,
    pub grid_height: i64,
    pub grid_size: i64,
}

#[derive(Clone, Debug)]
pub struct HydrateLdtkLevelDataTemp {
    pub layer_data: Vec<HydrateLdtkLayerDataTemp>,
    pub vertex_data: Option<hydrate_pipeline::AssetArtifactIdPair>,
    pub index_data: Option<hydrate_pipeline::AssetArtifactIdPair>,
}

#[derive(Clone, Debug)]
pub struct HydrateLdtkTileSetTemp {
    pub image: AssetId,
    pub material_instance: ArtifactId,
    pub image_width: u32,
    pub image_height: u32,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "1eb04266-4a32-473a-a54a-70b4d1172877"]
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
        _load_handle: LoadHandle,
    ) -> RafxResult<LdtkProjectAsset> {
        let mut levels = FnvHashMap::<LevelUid, LdtkLevel>::default();
        for (&level_uid, level_data) in &ldtk_asset.levels {
            let mut layers = Vec::default();
            for layer_data in &level_data.layer_data {
                let material_instance = asset_manager
                    .latest_asset(&layer_data.material_instance)
                    .unwrap();
                let _material_pass = material_instance
                    .material
                    .get_single_material_pass()
                    .expect("tileset material must have a single pass for opaque phase");
                let material_pass_index = 0;
                let tileset_image_set_index = tile_layer_frag::TEX_DESCRIPTOR_SET_INDEX;
                let descriptor_set = material_instance.material_descriptor_sets
                    [material_pass_index][tileset_image_set_index]
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
