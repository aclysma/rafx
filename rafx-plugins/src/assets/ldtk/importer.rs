use crate::assets::ldtk::{
    LdtkAssetData, LdtkLayerData, LdtkLayerDrawCallData, LdtkLevelData, LdtkTileSet, LevelUid,
};
use crate::features::tile_layer::TileLayerVertex;
use crate::schema::{LdtkAssetAccessor, LdtkAssetRecord, LdtkImportDataRecord};
use fnv::FnvHashMap;
use hydrate_base::{ArtifactId, AssetId, Handle};
use hydrate_data::{Record, RecordAccessor};
use hydrate_pipeline::{
    AssetPlugin, Builder, BuilderContext, BuilderRegistryBuilder, ImportContext, Importer,
    ImporterRegistryBuilder, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    PipelineResult, RunContext, ScanContext, SchemaLinker,
};
use ldtk_rust::{LayerInstance, Level, TileInstance};
use rafx::api::RafxResourceType;
use rafx::assets::{
    BufferAssetData, MaterialAsset, MaterialInstanceAssetData, MaterialInstanceSlotAssignment,
};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct HydrateLdtkTileSetTemp {
    pub image: AssetId,
    pub material_instance: ArtifactId,
    pub image_width: u32,
    pub image_height: u32,
}

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
pub struct LdtkImporter;

impl Importer for LdtkImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["ldtk"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let project: ldtk_rust::Project = serde_json::from_str(&source)?;

        let importable = context.add_default_importable::<LdtkAssetRecord>()?;

        for tileset in &project.defs.tilesets {
            importable.add_file_reference(&tileset.rel_path)?;
        }

        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let project: ldtk_rust::Project = serde_json::from_str(&source)?;

        //
        // Create the default asset
        //
        let default_asset = LdtkAssetRecord::new_builder(context.schema_set);

        let import_data = LdtkImportDataRecord::new_builder(context.schema_set);
        import_data.json_data().set(source)?;

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
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

    fn run<'a>(
        &self,
        context: &'a RunContext<'a, Self::InputT>,
    ) -> PipelineResult<LdtkJobOutput> {
        //
        // Read import data
        //
        let imported_data =
            context.imported_data::<LdtkImportDataRecord>(context.input.asset_id)?;

        let json_str = imported_data.json_data().get()?;
        let project: ldtk_rust::Project = serde_json::from_str(&json_str)?;

        let file_references = context
            .data_set
            .resolve_all_file_references(context.input.asset_id)?;

        // CPU-form of tileset data
        let mut tilesets_temp = FnvHashMap::default();

        // The one material we always use for tile layers
        //let material_handle = make_handle_from_str("ae8320e2-9d84-432d-879b-e34ebef90a82")?;

        for tileset in &project.defs.tilesets {
            //
            // Create a material instance
            //
            let image_object_id = file_references
                .get(&tileset.rel_path.as_str().into())
                .ok_or("Could not find asset ID assocaited with path")?;

            let material_instance_artifact_name = format!("mi_{}", tileset.uid);
            let material_instance_artifact_id = context.produce_artifact_with_handles(
                context.input.asset_id,
                Some(material_instance_artifact_name),
                |handle_factory| {
                    let material_handle: Handle<MaterialAsset> = handle_factory
                        .make_handle_to_default_artifact(AssetId::from_uuid(Uuid::parse_str(
                            "843a3b00-00d2-424f-94d8-629ca6060471",
                        )?));

                    let image_handle =
                        handle_factory.make_handle_to_default_artifact(*image_object_id);

                    let mut slot_assignments = vec![];
                    slot_assignments.push(MaterialInstanceSlotAssignment {
                        slot_name: "tilemap_texture".to_string(),
                        array_index: 0,
                        image: Some(image_handle.clone()),
                        sampler: None,
                        buffer_data: None,
                    });

                    Ok(MaterialInstanceAssetData {
                        material: material_handle.clone(),
                        slot_assignments,
                    })
                },
            )?;

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
                let vb_artifact = context.produce_artifact(
                    context.input.asset_id,
                    Some(format!("vertex_buffer,{:?}", level.uid)),
                    vertex_buffer_asset_data,
                )?;

                //
                // Create an index buffer for the level
                //
                let index_buffer_asset_data =
                    BufferAssetData::from_vec(RafxResourceType::INDEX_BUFFER, &index_data);
                let ib_artifact = context.produce_artifact(
                    context.input.asset_id,
                    Some(format!("index_buffer,{:?}", level.uid)),
                    index_buffer_asset_data,
                )?;

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

        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                let mut tilesets = FnvHashMap::default();
                for (uid, tileset) in tilesets_temp {
                    tilesets.insert(
                        uid,
                        LdtkTileSet {
                            material_instance: handle_factory.make_handle_to_artifact_raw(
                                context.input.asset_id,
                                tileset.material_instance,
                            ),
                            image: handle_factory.make_handle_to_default_artifact(tileset.image),
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
                            material_instance: handle_factory.make_handle_to_artifact_raw(
                                context.input.asset_id,
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
                                .map(|x| handle_factory.make_handle_to_artifact(x)),
                            index_data: level
                                .index_data
                                .map(|x| handle_factory.make_handle_to_artifact(x)),
                        },
                    );
                }

                Ok(LdtkAssetData { tilesets, levels })
            },
        )?;

        Ok(LdtkJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "a0cc5ab1-430c-4052-b082-074f63539fbe"]
pub struct LdtkBuilder {}

impl Builder for LdtkBuilder {
    fn asset_type(&self) -> &'static str {
        LdtkAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //Future: Might produce jobs per-platform
        context.enqueue_job::<LdtkJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            LdtkJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

pub struct LdtkAssetPlugin;

impl AssetPlugin for LdtkAssetPlugin {
    fn setup(
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<LdtkImporter>();
        builder_registry.register_handler::<LdtkBuilder>();
        job_processor_registry.register_job_processor::<LdtkJobProcessor>();
    }
}
