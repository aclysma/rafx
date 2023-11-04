use crate::assets::ldtk::{
    LdtkAssetData, LdtkLayerData, LdtkLayerDrawCallData, LdtkLevelData, LdtkTileSet, LevelUid,
    TileSetUid,
};
use crate::features::tile_layer::TileLayerVertex;
use crate::schema::{LdtkAssetRecord, LdtkImportDataRecord};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use fnv::FnvHashMap;
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, ImporterId, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable, SchemaLinker,
};
use itertools::Itertools;
use ldtk_rust::{LayerInstance, Level, TileInstance};
use rafx::api::RafxResourceType;
use rafx::assets::{
    BufferAssetData, GpuImageImporterSimple, HydrateMaterialInstanceAssetData,
    HydrateMaterialInstanceSlotAssignment, ImageAsset, MaterialAsset, MaterialInstanceAssetData,
    MaterialInstanceSlotAssignment,
};
use rafx::distill::importer::Error;
use rafx::distill::loader::handle::{Handle, SerdeContext};
use rafx::distill::loader::AssetRef;
use rafx::distill::{make_handle, make_handle_from_str};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use type_uuid::*;
use uuid::Uuid;

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
                    generate_draw_data(
                        level,
                        layer,
                        z_pos,
                        &layer.grid_tiles,
                        tileset,
                        &mut vertex_data,
                        &mut index_data,
                        &mut layer_draw_call_data,
                    );
                    generate_draw_data(
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
                        world_x_pos: level.world_x + layer.px_total_offset_x,
                        world_y_pos: level.world_y + layer.px_total_offset_y,
                        grid_width: layer.c_wid,
                        grid_height: layer.c_hei,
                        grid_size: layer.grid_size,
                    })
                }
            }

            let mut vertex_buffer_handle = None;
            let mut index_buffer_handle = None;

            if !vertex_data.is_empty() & !index_data.is_empty() {
                //
                // Create a vertex buffer for the level
                //
                let vertex_buffer_asset_data =
                    BufferAssetData::from_vec(RafxResourceType::VERTEX_BUFFER, &vertex_data);
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
                let index_buffer_asset_data =
                    BufferAssetData::from_vec(RafxResourceType::INDEX_BUFFER, &index_data);
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

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}

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

        let mut vertex_count = (layer_draw_call_data
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
                index_data_offset_in_bytes: (index_data.len() * std::mem::size_of::<u16>()) as u32,
                index_count: 0,
                z_pos,
            });

            vertex_count = 0;
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

#[derive(TypeUuid, Default)]
#[uuid = "7d507fac-ccb8-47fb-a4af-15da5e751601"]
pub struct HydrateLdtkImporter;

impl hydrate_model::Importer for HydrateLdtkImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["ldtk"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
    ) -> Vec<ScannedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(path).unwrap();
        let project: serde_json::Result<ldtk_rust::Project> = serde_json::from_str(&source);
        if let Err(err) = project {
            panic!("LDTK Import error: {:?}", err);
            //Err(Error::Boxed(Box::new(err))).unwrap();
        }

        let project = project.unwrap();

        let asset_type = schema_set
            .find_named_type(LdtkAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let image_importer_id = ImporterId(Uuid::from_bytes(GpuImageImporterSimple::UUID));

        for tileset in &project.defs.tilesets {
            file_references.push(ReferencedSourceFile {
                importer_id: image_importer_id,
                path: PathBuf::from_str(&tileset.rel_path).unwrap(),
            })
        }

        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references,
        }]
    }

    fn import_file(
        &self,
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(path).unwrap();
        let project: serde_json::Result<ldtk_rust::Project> = serde_json::from_str(&source);
        if let Err(err) = project {
            panic!("LDTK Import error: {:?}", err);
            //Err(Error::Boxed(Box::new(err))).unwrap();
        }

        let project = project.unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object = LdtkAssetRecord::new_single_object(schema_set).unwrap();
            // let mut default_asset_data_container =
            //     DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            // let x = LdtkAssetRecord::default();

            // No fields to write
            default_asset_object
        };

        let import_data = {
            let mut import_data_object =
                LdtkImportDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_data_container =
                DataContainerMut::new_single_object(&mut import_data_object, schema_set);
            let x = LdtkImportDataRecord::default();

            x.json_data()
                .set(&mut import_data_data_container, source)
                .unwrap();

            // No fields to write
            import_data_object
        };

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data),
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct LdtkJobInput {
    pub asset_id: ObjectId,
}
impl JobInput for LdtkJobInput {}

#[derive(Serialize, Deserialize)]
pub struct LdtkJobOutput {}
impl JobOutput for LdtkJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "a11b7361-33ae-4361-905b-fe25d2ac389e"]
pub struct LdtkJobProcessor;

impl JobProcessor for LdtkJobProcessor {
    type InputT = LdtkJobInput;
    type OutputT = LdtkJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        input: &LdtkJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies {
            import_data: vec![input.asset_id],
            upstream_jobs: Default::default(),
        }
    }

    fn run(
        &self,
        input: &LdtkJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> LdtkJobOutput {
        //
        // Read import data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::new_single_object(imported_data, schema_set);
        let x = LdtkImportDataRecord::default();

        let json_str = x.json_data().get(&data_container).unwrap();
        let project: serde_json::Result<ldtk_rust::Project> = serde_json::from_str(&json_str);
        if let Err(err) = project {
            panic!("LDTK Import error: {:?}", err);
            //Err(Error::Boxed(Box::new(err))).unwrap();
        }

        let project = project.unwrap();

        let file_references = data_set
            .resolve_all_file_references(input.asset_id)
            .unwrap();

        // All imported assets
        let mut imported_assets = Vec::<ImportedAsset>::default();
        // CPU-form of tileset data
        let mut tilesets = FnvHashMap::default();

        // The one material we always use for tile layers
        //let material_handle = make_handle_from_str("ae8320e2-9d84-432d-879b-e34ebef90a82")?;

        for tileset in &project.defs.tilesets {
            //
            // Get the image asset
            //
            // let asset_path = AssetRef::Path(tileset.rel_path.clone().into());
            // let image_handle = SerdeContext::with_active(|loader_info_provider, ref_op_sender| {
            //     let load_handle = loader_info_provider.get_load_handle(&asset_path).unwrap();
            //     Handle::<ImageAsset>::new(ref_op_sender.clone(), load_handle)
            // });

            //
            // Create a material instance
            //
            let material_instance_artifact_name = format!("mi_{}", tileset.uid);
            let material_instance_artifact = job_system::produce_artifact_with_handles(
                job_api,
                input.asset_id,
                Some(material_instance_artifact_name),
                || {
                    let material_handle: hydrate_base::Handle<MaterialAsset> =
                        job_system::make_handle_to_default_artifact(
                            job_api,
                            ObjectId::from_uuid(
                                Uuid::parse_str("843a3b00-00d2-424f-94d8-629ca6060471").unwrap(),
                            ),
                        );

                    let image_object_id = file_references
                        .get(&PathBuf::from_str(&tileset.rel_path).unwrap())
                        .unwrap();
                    let image_handle =
                        job_system::make_handle_to_default_artifact(job_api, *image_object_id);

                    let mut slot_assignments = vec![];
                    slot_assignments.push(HydrateMaterialInstanceSlotAssignment {
                        slot_name: "tilemap_texture".to_string(),
                        array_index: 0,
                        image: Some(image_handle.clone()),
                        sampler: None,
                        buffer_data: None,
                    });

                    HydrateMaterialInstanceAssetData {
                        material: material_handle.clone(),
                        slot_assignments,
                    }
                },
            );

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
                    generate_draw_data(
                        level,
                        layer,
                        z_pos,
                        &layer.grid_tiles,
                        tileset,
                        &mut vertex_data,
                        &mut index_data,
                        &mut layer_draw_call_data,
                    );
                    generate_draw_data(
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
                        world_x_pos: level.world_x + layer.px_total_offset_x,
                        world_y_pos: level.world_y + layer.px_total_offset_y,
                        grid_width: layer.c_wid,
                        grid_height: layer.c_hei,
                        grid_size: layer.grid_size,
                    })
                }
            }

            let mut vertex_buffer_handle = None;
            let mut index_buffer_handle = None;

            if !vertex_data.is_empty() & !index_data.is_empty() {
                //
                // Create a vertex buffer for the level
                //
                let vertex_buffer_asset_data =
                    BufferAssetData::from_vec(RafxResourceType::VERTEX_BUFFER, &vertex_data);
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
                let index_buffer_asset_data =
                    BufferAssetData::from_vec(RafxResourceType::INDEX_BUFFER, &index_data);
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

        /*
                job_system::produce_asset_with_handles(job_api, input.asset_id, || {
                    let mut objects = Vec::with_capacity(json_format.objects.len());
                    for json_object in json_format.objects {
                        let model = if let Some(json_model) = &json_object.model {
                            let model_object_id = file_references.get(&json_model.model).unwrap();
                            let model_handle =
                                job_system::make_handle_to_default_artifact(job_api, *model_object_id);

                            Some(HydratePrefabAdvAssetDataObjectModel {
                                model: model_handle,
                            })
                        } else {
                            None
                        };

                        let light = if let Some(json_light) = &json_object.light {
                            let light = json_light.clone();
                            let spot = light
                                .spot
                                .as_ref()
                                .map(|x| PrefabAdvAssetDataObjectLightSpot {
                                    inner_angle: x.inner_angle,
                                    outer_angle: x.outer_angle,
                                });

                            let range = if light.cutoff_distance.unwrap_or(-1.0) < 0.0 {
                                None
                            } else {
                                light.cutoff_distance
                            };
                            Some(PrefabAdvAssetDataObjectLight {
                                color: light.color.into(),
                                kind: light.kind.into(),
                                intensity: light.intensity,
                                range,
                                spot,
                            })
                        } else {
                            None
                        };

                        let transform = PrefabAdvAssetDataObjectTransform {
                            position: json_object.transform.position.into(),
                            rotation: json_object.transform.rotation.into(),
                            scale: json_object.transform.scale.into(),
                        };

                        objects.push(HydratePrefabAdvAssetDataObject {
                            transform,
                            model,
                            light,
                        });
                    }

                    HydratePrefabAdvAssetData { objects }
                });
        */
        LdtkJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "02a99fa5-8053-4d6f-9e8d-54cb9e2f7949"]
pub struct LdtkBuilder {}

impl hydrate_model::Builder for LdtkBuilder {
    fn asset_type(&self) -> &'static str {
        LdtkAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::new_dataset(data_set, schema_set, asset_id);
        //let x = LdtkAssetRecord::default();

        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<LdtkJobProcessor>(
            data_set,
            schema_set,
            job_api,
            LdtkJobInput { asset_id },
        );
    }
}

pub struct LdtkAssetPlugin;

impl hydrate_model::AssetPlugin for LdtkAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<HydrateLdtkImporter>(schema_linker);
        builder_registry.register_handler::<LdtkBuilder>(schema_linker);
        job_processor_registry.register_job_processor::<LdtkJobProcessor>();
    }
}
