use crate::assets::ldtk::{
    HydrateLdtkLayerDataTemp, HydrateLdtkLevelDataTemp, HydrateLdtkTileSetTemp, LdtkAssetData,
    LdtkLayerData, LdtkLayerDrawCallData, LdtkLevelData, LdtkTileSet, LevelUid,
};
use crate::features::tile_layer::TileLayerVertex;
use crate::schema::{LdtkAssetRecord, LdtkImportDataRecord};
use fnv::FnvHashMap;
use hydrate_base::hashing::HashMap;
use hydrate_base::{AssetId, Handle};
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, ImporterId, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableAsset, ImportedImportable, ImporterRegistry,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable, SchemaLinker,
};
use ldtk_rust::{LayerInstance, Level, TileInstance};
use rafx::api::RafxResourceType;
use rafx::assets::{
    BufferAssetData, GpuImageImporterSimple, MaterialAsset, MaterialInstanceAssetData,
    MaterialInstanceSlotAssignment,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use type_uuid::*;
use uuid::Uuid;

fn generate_draw_data(
    level: &Level,
    layer: &LayerInstance,
    z_pos: f32,
    tile_instances: &[TileInstance],
    tileset: &HydrateLdtkTileSetTemp,
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
        _importer_registry: &ImporterRegistry,
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
        importable_assets: &HashMap<Option<String>, ImportableAsset>,
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

        project.unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object = LdtkAssetRecord::new_single_object(schema_set).unwrap();
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
    pub asset_id: AssetId,
}
impl JobInput for LdtkJobInput {}

#[derive(Serialize, Deserialize)]
pub struct LdtkJobOutput {}
impl JobOutput for LdtkJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "2e4e713e-71ef-4972-bb6b-827a4d291ccb"]
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
        _data_set: &DataSet,
        _schema_set: &SchemaSet,
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
        dependency_data: &HashMap<AssetId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> LdtkJobOutput {
        //
        // Read import data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::from_single_object(imported_data, schema_set);
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

        // CPU-form of tileset data
        let mut tilesets_temp = FnvHashMap::default();

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
            let image_object_id = file_references
                .get(&PathBuf::from_str(&tileset.rel_path).unwrap())
                .unwrap();

            let material_instance_artifact_name = format!("mi_{}", tileset.uid);
            let material_instance_artifact_id = job_system::produce_artifact_with_handles(
                job_api,
                input.asset_id,
                Some(material_instance_artifact_name),
                || {
                    let material_handle: Handle<MaterialAsset> =
                        job_system::make_handle_to_default_artifact(
                            job_api,
                            AssetId::from_uuid(
                                Uuid::parse_str("843a3b00-00d2-424f-94d8-629ca6060471").unwrap(),
                            ),
                        );

                    let image_handle =
                        job_system::make_handle_to_default_artifact(job_api, *image_object_id);

                    let mut slot_assignments = vec![];
                    slot_assignments.push(MaterialInstanceSlotAssignment {
                        slot_name: "tilemap_texture".to_string(),
                        array_index: 0,
                        image: Some(image_handle.clone()),
                        sampler: None,
                        buffer_data: None,
                    });

                    MaterialInstanceAssetData {
                        material: material_handle.clone(),
                        slot_assignments,
                    }
                },
            );

            let image_width = tileset.px_wid as _;
            let image_height = tileset.px_hei as _;

            tilesets_temp.insert(
                tileset.uid,
                HydrateLdtkTileSetTemp {
                    image: *image_object_id,
                    material_instance: material_instance_artifact_id,
                    image_width,
                    image_height,
                },
            );
        }

        let mut levels_temp = FnvHashMap::<LevelUid, HydrateLdtkLevelDataTemp>::default();

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
                    let tileset = &tilesets_temp[&tileset_uid];

                    let mut layer_draw_call_data: Vec<LdtkLayerDrawCallData> = Vec::default();

                    //TODO: Data drive this from the asset
                    let z_pos = ((level.layer_instances.as_ref().unwrap().len() - layer_index - 1)
                        * 10) as f32;

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

                    layer_data.push(HydrateLdtkLayerDataTemp {
                        material_instance: tileset.material_instance,
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

            let mut vertex_buffer_artifact = None;
            let mut index_buffer_artifact = None;

            if !vertex_data.is_empty() & !index_data.is_empty() {
                //
                // Create a vertex buffer for the level
                //
                let vertex_buffer_asset_data =
                    BufferAssetData::from_vec(RafxResourceType::VERTEX_BUFFER, &vertex_data);
                let vb_artifact = job_system::produce_artifact(
                    job_api,
                    input.asset_id,
                    Some(format!("vertex_buffer,{:?}", level.uid)),
                    vertex_buffer_asset_data,
                );

                //
                // Create an index buffer for the level
                //
                let index_buffer_asset_data =
                    BufferAssetData::from_vec(RafxResourceType::INDEX_BUFFER, &index_data);
                let ib_artifact = job_system::produce_artifact(
                    job_api,
                    input.asset_id,
                    Some(format!("index_buffer,{:?}", level.uid)),
                    index_buffer_asset_data,
                );

                vertex_buffer_artifact = Some(vb_artifact);
                index_buffer_artifact = Some(ib_artifact);
            }

            let old = levels_temp.insert(
                level.uid,
                HydrateLdtkLevelDataTemp {
                    layer_data,
                    vertex_data: vertex_buffer_artifact,
                    index_data: index_buffer_artifact,
                },
            );
            assert!(old.is_none());
        }

        job_system::produce_asset_with_handles(job_api, input.asset_id, || {
            let mut tilesets = FnvHashMap::default();
            for (uid, tileset) in tilesets_temp {
                tilesets.insert(
                    uid,
                    LdtkTileSet {
                        material_instance: job_system::make_handle_to_artifact_raw(
                            job_api,
                            input.asset_id,
                            tileset.material_instance,
                        ),
                        image: job_system::make_handle_to_default_artifact(job_api, tileset.image),
                        image_width: tileset.image_width,
                        image_height: tileset.image_height,
                    },
                );
            }

            let mut levels = FnvHashMap::default();
            for (uid, level) in levels_temp {
                let layers = level
                    .layer_data
                    .into_iter()
                    .map(|layer| LdtkLayerData {
                        material_instance: job_system::make_handle_to_artifact_raw(
                            job_api,
                            input.asset_id,
                            layer.material_instance,
                        ),
                        draw_call_data: layer.draw_call_data,
                        z_pos: layer.z_pos,
                        world_x_pos: layer.world_x_pos,
                        world_y_pos: layer.world_y_pos,
                        grid_width: layer.grid_width,
                        grid_height: layer.grid_height,
                        grid_size: layer.grid_size,
                    })
                    .collect();

                levels.insert(
                    uid,
                    LdtkLevelData {
                        layer_data: layers,
                        vertex_data: level
                            .vertex_data
                            .map(|x| job_system::make_handle_to_artifact(job_api, x)),
                        index_data: level
                            .index_data
                            .map(|x| job_system::make_handle_to_artifact(job_api, x)),
                    },
                );
            }

            LdtkAssetData { tilesets, levels }
        });

        LdtkJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "a0cc5ab1-430c-4052-b082-074f63539fbe"]
pub struct LdtkBuilder {}

impl hydrate_model::Builder for LdtkBuilder {
    fn asset_type(&self) -> &'static str {
        LdtkAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: AssetId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::from_dataset(data_set, schema_set, asset_id);
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
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<HydrateLdtkImporter>();
        builder_registry.register_handler::<LdtkBuilder>();
        job_processor_registry.register_job_processor::<LdtkJobProcessor>();
    }
}
