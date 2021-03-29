use crate::assets::ldtk::{
    LdtkAssetData, LdtkLayerData, LdtkLayerDrawCallData, LdtkLevelData, LdtkTileSet, LevelUid,
    TileSetUid,
};
use crate::features::tile_layer::TileLayerVertex;
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use fnv::FnvHashMap;
use itertools::Itertools;
use ldtk_rust::{LayerInstance, Level, TileInstance};
use rafx::assets::{
    BufferAssetData, ImageAsset, MaterialInstanceAssetData, MaterialInstanceSlotAssignment,
};
use rafx::distill::importer::Error;
use rafx::distill::loader::handle::{Handle, SerdeContext};
use rafx::distill::loader::AssetRef;
use rafx::distill::{make_handle, make_handle_from_str};
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Default, Clone, Debug)]
#[uuid = "84510429-7e8f-403a-ae51-4defcffe00fb"]
pub struct LdtkImporterOptions {
    layer_z_positions: Vec<f32>,
}

// The asset state is stored in this format using Vecs
#[derive(TypeUuid, Serialize, Deserialize, Default, Clone, Debug)]
#[uuid = "74c12ab4-c836-48ab-b6e8-3e483be60dcf"]
pub struct LdtkImporterStateStable {
    // Asset UUIDs for imported image by name. We use vecs here so we can sort by UUID for
    // deterministic output
    material_instance_uuids: Vec<(TileSetUid, AssetUuid)>,
    level_vertex_buffer_uuids: Vec<(LevelUid, AssetUuid)>,
    level_index_buffer_uuids: Vec<(LevelUid, AssetUuid)>,
    ldtk_asset_uuid: Option<AssetUuid>,
}

impl From<LdtkImporterStateUnstable> for LdtkImporterStateStable {
    fn from(other: LdtkImporterStateUnstable) -> Self {
        let mut stable = LdtkImporterStateStable::default();
        stable.material_instance_uuids = other
            .material_instance_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.level_vertex_buffer_uuids = other
            .level_vertex_buffer_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.level_index_buffer_uuids = other
            .level_index_buffer_uuids
            .into_iter()
            .sorted_by_key(|(id, _uuid)| id.clone())
            .collect();
        stable.ldtk_asset_uuid = other.ldtk_asset_uuid.clone();
        stable
    }
}

#[derive(Default)]
pub struct LdtkImporterStateUnstable {
    material_instance_uuids: FnvHashMap<TileSetUid, AssetUuid>,
    level_vertex_buffer_uuids: FnvHashMap<LevelUid, AssetUuid>,
    level_index_buffer_uuids: FnvHashMap<LevelUid, AssetUuid>,
    ldtk_asset_uuid: Option<AssetUuid>,
}

impl From<LdtkImporterStateStable> for LdtkImporterStateUnstable {
    fn from(other: LdtkImporterStateStable) -> Self {
        let mut unstable = LdtkImporterStateUnstable::default();
        unstable.material_instance_uuids = other.material_instance_uuids.into_iter().collect();
        unstable.level_vertex_buffer_uuids = other.level_vertex_buffer_uuids.into_iter().collect();
        unstable.level_index_buffer_uuids = other.level_index_buffer_uuids.into_iter().collect();
        unstable.ldtk_asset_uuid = other.ldtk_asset_uuid.clone();
        unstable
    }
}

#[derive(TypeUuid)]
#[uuid = "dd701f1b-df5c-44e4-bd7b-0456a6c0aa47"]
pub struct LdtkImporter;
impl Importer for LdtkImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        4
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = LdtkImporterOptions;

    type State = LdtkImporterStateStable;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        op: &mut ImportOp,
        source: &mut dyn Read,
        options: &Self::Options,
        stable_state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let mut unstable_state: LdtkImporterStateUnstable = stable_state.clone().into();

        //
        // Assign an ID to this ldtk file if not already assigned
        //
        unstable_state.ldtk_asset_uuid = Some(
            unstable_state
                .ldtk_asset_uuid
                .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes())),
        );

        // Read in the LDTK file
        let project: serde_json::Result<ldtk_rust::Project> = serde_json::from_reader(source);
        if let Err(err) = project {
            log::error!("LDTK Import error: {:?}", err);
            return Err(Error::Boxed(Box::new(err)));
        }

        let project = project.unwrap();

        // All imported assets
        let mut imported_assets = Vec::<ImportedAsset>::default();
        // CPU-form of tileset data
        let mut tilesets = FnvHashMap::default();

        // The one material we always use for tile layers
        let material_handle = make_handle_from_str("ae8320e2-9d84-432d-879b-e34ebef90a82")?;

        for tileset in &project.defs.tilesets {
            //
            // Get the image asset
            //
            let asset_path = AssetRef::Path(tileset.rel_path.clone().into());
            let image_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
                let load_handle = loader_info_provider.get_load_handle(&asset_path).unwrap();
                Handle::<ImageAsset>::new(ref_op_sender.clone(), load_handle)
            });

            //
            // Create a material instance
            //
            let material_instance_uuid = *unstable_state
                .material_instance_uuids
                .entry(tileset.uid)
                .or_insert_with(|| op.new_asset_uuid());

            let material_instance_handle = make_handle(material_instance_uuid);

            let mut slot_assignments = vec![];
            slot_assignments.push(MaterialInstanceSlotAssignment {
                slot_name: "tilemap_texture".to_string(),
                array_index: 0,
                image: Some(image_handle.clone()),
                sampler: None,
                buffer_data: None,
            });

            let material_instance = MaterialInstanceAssetData {
                material: material_handle.clone(),
                slot_assignments,
            };

            //
            // Add material instance to list of imported assets
            //
            imported_assets.push(ImportedAsset {
                id: material_instance_uuid,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(material_instance),
            });

            let image_width = tileset.px_wid as _;
            let image_height = tileset.px_hei as _;

            tilesets.insert(
                tileset.uid,
                LdtkTileSet {
                    image: image_handle,
                    material_instance: material_instance_handle,
                    image_width,
                    image_height,
                },
            );
        }

        let mut levels = FnvHashMap::<LevelUid, LdtkLevelData>::default();

        for level in &project.levels {
            let mut vertex_data = Vec::<TileLayerVertex>::default();
            let mut index_data = Vec::<u16>::default();

            let mut layer_data = Vec::default();

            //TODO: Support for levels in separate files
            for (layer_index, layer) in level.layer_instances.as_ref().unwrap().iter().enumerate() {
                let tileset_uid = if let Some(tileset_uid) = layer.override_tileset_uid {
                    Some(tileset_uid)
                } else if let Some(tileset_uid) = layer.tileset_def_uid {
                    Some(tileset_uid)
                } else {
                    None
                };

                if let Some(tileset_uid) = tileset_uid {
                    let tileset = &tilesets[&tileset_uid];

                    let mut layer_draw_call_data: Vec<LdtkLayerDrawCallData> = Vec::default();

                    let z_pos = options
                        .layer_z_positions
                        .get(layer_index)
                        .copied()
                        .unwrap_or(layer_index as f32);
                    LdtkImporter::generate_draw_data(
                        level,
                        layer,
                        z_pos,
                        &layer.grid_tiles,
                        tileset,
                        &mut vertex_data,
                        &mut index_data,
                        &mut layer_draw_call_data,
                    );
                    LdtkImporter::generate_draw_data(
                        level,
                        layer,
                        z_pos,
                        &layer.auto_layer_tiles,
                        tileset,
                        &mut vertex_data,
                        &mut index_data,
                        &mut layer_draw_call_data,
                    );

                    layer_data.push(LdtkLayerData {
                        material_instance: tileset.material_instance.clone(),
                        draw_call_data: layer_draw_call_data,
                        z_pos,
                    })
                }
            }

            let mut vertex_buffer_handle = None;
            let mut index_buffer_handle = None;

            if !vertex_data.is_empty() & !index_data.is_empty() {
                //
                // Create a vertex buffer for the level
                //
                let vertex_buffer_asset_data = BufferAssetData::from_vec(&vertex_data);
                let vertex_buffer_uuid = *unstable_state
                    .level_vertex_buffer_uuids
                    .entry(level.uid)
                    .or_insert_with(|| op.new_asset_uuid());

                imported_assets.push(ImportedAsset {
                    id: vertex_buffer_uuid,
                    search_tags: vec![],
                    build_deps: vec![],
                    load_deps: vec![],
                    build_pipeline: None,
                    asset_data: Box::new(vertex_buffer_asset_data),
                });

                //
                // Create an index buffer for the level
                //
                let index_buffer_asset_data = BufferAssetData::from_vec(&index_data);
                let index_buffer_uuid = *unstable_state
                    .level_index_buffer_uuids
                    .entry(level.uid)
                    .or_insert_with(|| op.new_asset_uuid());

                imported_assets.push(ImportedAsset {
                    id: index_buffer_uuid,
                    search_tags: vec![],
                    build_deps: vec![],
                    load_deps: vec![],
                    build_pipeline: None,
                    asset_data: Box::new(index_buffer_asset_data),
                });

                vertex_buffer_handle = Some(make_handle(vertex_buffer_uuid));
                index_buffer_handle = Some(make_handle(index_buffer_uuid));
            }

            let old = levels.insert(
                level.uid,
                LdtkLevelData {
                    layer_data,
                    vertex_data: vertex_buffer_handle,
                    index_data: index_buffer_handle,
                },
            );
            assert!(old.is_none());
        }

        let asset_data = LdtkAssetData { tilesets, levels };

        imported_assets.push(ImportedAsset {
            id: unstable_state.ldtk_asset_uuid.unwrap(),
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(asset_data),
        });

        *stable_state = unstable_state.into();

        println!("state: {:#?}", stable_state);
        println!("imported assets: {}", imported_assets.len());

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}

impl LdtkImporter {
    fn generate_draw_data(
        level: &Level,
        layer: &LayerInstance,
        z_pos: f32,
        tile_instances: &[TileInstance],
        tileset: &LdtkTileSet,
        vertex_data: &mut Vec<TileLayerVertex>,
        index_data: &mut Vec<u16>,
        layer_draw_call_data: &mut Vec<LdtkLayerDrawCallData>,
    ) {
        for tile in tile_instances {
            //
            // If the vertex count exceeds what a u16 index buffer support, start a new draw call
            //

            let vertex_count = (layer_draw_call_data
                .last()
                .map(|x| x.index_count)
                .unwrap_or(0)
                / 6)
                * 4;
            if layer_draw_call_data.is_empty() || vertex_count + 4 > std::u16::MAX as u32 {
                layer_draw_call_data.push(LdtkLayerDrawCallData {
                    vertex_data_offset_in_bytes: (vertex_data.len()
                        * std::mem::size_of::<TileLayerVertex>())
                        as u32,
                    index_data_offset_in_bytes: (index_data.len() * std::mem::size_of::<u16>())
                        as u32,
                    index_count: 0,
                    z_pos,
                });
            }

            let current_draw_call_data = layer_draw_call_data.last_mut().unwrap();

            let flip_bits = tile.f;
            let x_pos = (tile.px[0] + layer.px_total_offset_x + level.world_x) as f32;
            let y_pos = -1.0 * (tile.px[1] + layer.px_total_offset_y + level.world_y) as f32;
            let tileset_src_x_pos = tile.src[0];
            let tileset_src_y_pos = tile.src[1];
            let tile_width = layer.grid_size as f32;
            let tile_height = layer.grid_size as f32;

            let mut texture_rect_left = tileset_src_x_pos as f32 / tileset.image_width as f32;
            let mut texture_rect_right =
                (tileset_src_x_pos as f32 + tile_width) / tileset.image_width as f32;
            let mut texture_rect_top =
                (tileset_src_y_pos as f32 + tile_height) / tileset.image_height as f32;
            let mut texture_rect_bottom = (tileset_src_y_pos as f32) / tileset.image_height as f32;

            //
            // Handle flipping the image
            //
            if (flip_bits & 1) == 1 {
                std::mem::swap(&mut texture_rect_left, &mut texture_rect_right);
            }

            if (flip_bits & 2) == 2 {
                std::mem::swap(&mut texture_rect_top, &mut texture_rect_bottom);
            }

            //
            // Insert vertex data
            //
            vertex_data.push(TileLayerVertex {
                position: [x_pos + tile_width, y_pos + tile_height, z_pos],
                uv: [texture_rect_right, texture_rect_bottom],
            });
            vertex_data.push(TileLayerVertex {
                position: [x_pos, y_pos + tile_height, z_pos],
                uv: [texture_rect_left, texture_rect_bottom],
            });
            vertex_data.push(TileLayerVertex {
                position: [x_pos + tile_width, y_pos, z_pos],
                uv: [texture_rect_right, texture_rect_top],
            });
            vertex_data.push(TileLayerVertex {
                position: [x_pos, y_pos, z_pos],
                uv: [texture_rect_left, texture_rect_top],
            });

            //
            // Insert index data
            //
            index_data.push(vertex_count as u16 + 0);
            index_data.push(vertex_count as u16 + 1);
            index_data.push(vertex_count as u16 + 2);
            index_data.push(vertex_count as u16 + 2);
            index_data.push(vertex_count as u16 + 1);
            index_data.push(vertex_count as u16 + 3);

            //
            // Update the draw call to include the new data
            //
            current_draw_call_data.index_count += 6;
        }
    }
}
