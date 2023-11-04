use crate::assets::mesh_adv::{
    BlenderMaterialImporter, MeshAdvAssetData, MeshAdvBufferAssetData, MeshAdvPartAssetData,
    MeshMaterialAdvAsset,
};
use crate::features::mesh_adv::{MeshVertexFull, MeshVertexPosition};
use crate::schema::{MeshAdvMeshAssetRecord, MeshAdvMeshImportedDataRecord};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use glam::Vec3;
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerMut, ImporterId, Record, SchemaSet};
use hydrate_model::{
    AssetPlugin, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistryBuilder, JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable,
    SchemaLinker,
};
use rafx::api::RafxResourceType;
use rafx::assets::PushBuffer;
use rafx::base::b3f::B3FReader;
use rafx::distill::loader::handle::Handle;
use rafx::distill::make_handle;
use rafx::rafx_visibility::{PolygonSoup, PolygonSoupIndex, VisibleBounds};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use type_uuid::*;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
enum MeshPartJsonIndexType {
    U16,
    U32,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshPartJson {
    #[serde(default)]
    pub position: Option<u32>,
    #[serde(default)]
    pub normal: Option<u32>,
    #[serde(default)]
    pub tangent: Option<u32>,
    #[serde(default)]
    pub uv: Vec<u32>,
    pub indices: u32,
    pub index_type: MeshPartJsonIndexType,
    pub material: Handle<MeshMaterialAdvAsset>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HydrateMeshPartJson {
    #[serde(default)]
    pub position: Option<u32>,
    #[serde(default)]
    pub normal: Option<u32>,
    #[serde(default)]
    pub tangent: Option<u32>,
    #[serde(default)]
    pub uv: Vec<u32>,
    pub indices: u32,
    pub index_type: MeshPartJsonIndexType,
    pub material: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct HydrateMeshJson {
    pub mesh_parts: Vec<HydrateMeshPartJson>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshJson {
    pub mesh_parts: Vec<MeshPartJson>,
}

fn try_cast_u8_slice<T: Copy + 'static>(data: &[u8]) -> Option<&[T]> {
    if data.len() % std::mem::size_of::<T>() != 0 {
        return None;
    }

    let ptr = data.as_ptr() as *const T;
    if ptr as usize % std::mem::align_of::<T>() != 0 {
        return None;
    }

    let casted: &[T] =
        unsafe { std::slice::from_raw_parts(ptr, data.len() / std::mem::size_of::<T>()) };

    Some(casted)
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "b824818f-7026-412f-ba88-32bc25c1c7f4"]
pub struct MeshAdvBlenderImporterState {
    mesh_id: Option<AssetUuid>,
    vertex_full_buffer_id: Option<AssetUuid>,
    vertex_position_buffer_id: Option<AssetUuid>,
    index_buffer_id: Option<AssetUuid>,
}

#[derive(TypeUuid)]
#[uuid = "5f2be1a1-b025-4d72-960b-24cb03ff19de"]
pub struct MeshAdvBlenderImporter;
impl Importer for MeshAdvBlenderImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        6
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MeshAdvBlenderImporterState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let mesh_id = state
            .mesh_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        let vertex_full_buffer_id = state
            .vertex_full_buffer_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        let vertex_position_buffer_id = state
            .vertex_position_buffer_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        let index_buffer_id = state
            .index_buffer_id
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = MeshAdvBlenderImporterState {
            mesh_id: Some(mesh_id),
            vertex_full_buffer_id: Some(vertex_full_buffer_id),
            vertex_position_buffer_id: Some(vertex_position_buffer_id),
            index_buffer_id: Some(index_buffer_id),
        };
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let b3f_reader = rafx::base::b3f::B3FReader::new(&bytes)
            .ok_or("Blender Mesh Import error, mesh file format not recognized")?;
        let mesh_as_json: MeshJson =
            serde_json::from_slice(b3f_reader.get_block(0)).map_err(|e| e.to_string())?;

        let mut all_positions = Vec::<glam::Vec3>::with_capacity(1024);
        let mut all_position_indices = Vec::<u32>::with_capacity(8192);

        let mut all_vertices_full = PushBuffer::new(16384);
        let mut all_vertices_position = PushBuffer::new(16384);
        let mut all_indices = PushBuffer::new(16384);

        let mut mesh_parts: Vec<MeshAdvPartAssetData> =
            Vec::with_capacity(mesh_as_json.mesh_parts.len());

        for mesh_part in &mesh_as_json.mesh_parts {
            //
            // Get byte slices of all input data for this mesh part
            //
            let positions_bytes =
                b3f_reader.get_block(mesh_part.position.ok_or("No position data")? as usize);
            let normals_bytes =
                b3f_reader.get_block(mesh_part.normal.ok_or("No normal data")? as usize);
            let tex_coords_bytes = b3f_reader
                .get_block(*mesh_part.uv.get(0).ok_or("No texture coordinate data")? as usize);
            let part_indices_bytes = b3f_reader.get_block(mesh_part.indices as usize);

            //
            // Get strongly typed slices of all input data for this mesh part
            //
            let positions = try_cast_u8_slice::<[f32; 3]>(positions_bytes)
                .ok_or("Could not cast due to alignment")?;
            let normals = try_cast_u8_slice::<[f32; 3]>(normals_bytes)
                .ok_or("Could not cast due to alignment")?;
            let tex_coords = try_cast_u8_slice::<[f32; 2]>(tex_coords_bytes)
                .ok_or("Could not cast due to alignment")?;

            // Indices may be encoded as u16 or u32, either way copy them out to a Vec<u32>
            let mut part_indices = Vec::<u32>::default();
            match mesh_part.index_type {
                MeshPartJsonIndexType::U16 => {
                    let part_indices_u16 = try_cast_u8_slice::<u16>(part_indices_bytes)
                        .ok_or("Could not cast due to alignment")?;
                    part_indices.reserve(part_indices_u16.len());
                    for &part_index in part_indices_u16 {
                        part_indices.push(part_index as u32);
                    }
                }
                MeshPartJsonIndexType::U32 => {
                    let part_indices_u32 = try_cast_u8_slice::<u32>(part_indices_bytes)
                        .ok_or("Could not cast due to alignment")?;
                    part_indices.reserve(part_indices_u32.len());
                    for &part_index in part_indices_u32 {
                        part_indices.push(part_index);
                    }
                }
            };

            let part_data = super::mesh_util::process_mesh_part(
                &part_indices,
                &positions,
                &normals,
                &tex_coords,
                &mut all_vertices_full,
                &mut all_vertices_position,
                &mut all_indices,
            );

            //
            // Positions and indices for the visibility system
            //
            for index in part_indices {
                all_position_indices.push(index as u32);
            }

            for i in 0..positions.len() {
                all_positions.push(Vec3::new(positions[i][0], positions[i][1], positions[i][2]));
            }

            let mesh_material = mesh_part.material.clone();

            mesh_parts.push(MeshAdvPartAssetData {
                mesh_material,
                vertex_full_buffer_offset_in_bytes: part_data.vertex_full_buffer_offset_in_bytes,
                vertex_full_buffer_size_in_bytes: part_data.vertex_full_buffer_size_in_bytes,
                vertex_position_buffer_offset_in_bytes: part_data
                    .vertex_position_buffer_offset_in_bytes,
                vertex_position_buffer_size_in_bytes: part_data
                    .vertex_position_buffer_size_in_bytes,
                index_buffer_offset_in_bytes: part_data.index_buffer_offset_in_bytes,
                index_buffer_size_in_bytes: part_data.index_buffer_size_in_bytes,
                index_type: part_data.index_type,
            })
        }

        let mut imported_assets = Vec::with_capacity(3);

        //
        // Vertex Full Buffer
        //
        assert!(!all_vertices_full.is_empty());
        let vertex_full_buffer_asset = MeshAdvBufferAssetData {
            resource_type: RafxResourceType::VERTEX_BUFFER,
            alignment: std::mem::size_of::<MeshVertexFull>() as u32,
            data: all_vertices_full.into_data(),
        };

        let vertex_full_buffer_handle = make_handle(vertex_full_buffer_id);

        //
        // Vertex Position Buffer
        //
        assert!(!all_vertices_position.is_empty());
        let vertex_position_buffer_asset = MeshAdvBufferAssetData {
            resource_type: RafxResourceType::VERTEX_BUFFER,
            alignment: std::mem::size_of::<MeshVertexPosition>() as u32,
            data: all_vertices_position.into_data(),
        };

        let vertex_position_buffer_handle = make_handle(vertex_position_buffer_id);

        //
        // Index Buffer
        //
        assert!(!all_indices.is_empty());
        let index_buffer_asset = MeshAdvBufferAssetData {
            resource_type: RafxResourceType::INDEX_BUFFER,
            alignment: std::mem::size_of::<u32>() as u32,
            data: all_indices.into_data(),
        };

        let index_buffer_handle = make_handle(index_buffer_id);

        let mesh_data = PolygonSoup {
            vertex_positions: all_positions,
            index: PolygonSoupIndex::Indexed32(all_position_indices),
        };

        let asset_data = MeshAdvAssetData {
            mesh_parts,
            vertex_full_buffer: vertex_full_buffer_handle,
            vertex_position_buffer: vertex_position_buffer_handle,
            index_buffer: index_buffer_handle,
            visible_bounds: VisibleBounds::from(mesh_data),
        };

        imported_assets.push(ImportedAsset {
            id: mesh_id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(asset_data),
        });

        imported_assets.push(ImportedAsset {
            id: vertex_full_buffer_id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(vertex_full_buffer_asset),
        });

        imported_assets.push(ImportedAsset {
            id: vertex_position_buffer_id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(vertex_position_buffer_asset),
        });

        imported_assets.push(ImportedAsset {
            id: index_buffer_id,
            search_tags: vec![],
            build_deps: vec![],
            load_deps: vec![],
            build_pipeline: None,
            asset_data: Box::new(index_buffer_asset),
        });

        Ok(ImporterValue {
            assets: imported_assets,
        })
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "bdd126da-2f3d-4cbb-b2f2-80088c715753"]
pub struct BlenderMeshImporter;

impl hydrate_model::Importer for BlenderMeshImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_mesh"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
    ) -> Vec<ScannedImportable> {
        let mesh_adv_asset_type = schema_set
            .find_named_type(MeshAdvMeshAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let bytes = std::fs::read(path).unwrap();

        let b3f_reader = B3FReader::new(&bytes)
            .ok_or("Blender Mesh Import error, mesh file format not recognized")
            .unwrap();
        let mesh_as_json: HydrateMeshJson = serde_json::from_slice(b3f_reader.get_block(0))
            .map_err(|e| e.to_string())
            .unwrap();

        fn try_add_file_reference<T: TypeUuid>(
            file_references: &mut Vec<ReferencedSourceFile>,
            path: PathBuf,
        ) {
            let importer_image_id = ImporterId(Uuid::from_bytes(T::UUID));
            file_references.push(ReferencedSourceFile {
                importer_id: importer_image_id,
                path,
            })
        }

        let mut mesh_file_references = Vec::default();
        for mesh_part in &mesh_as_json.mesh_parts {
            try_add_file_reference::<BlenderMaterialImporter>(
                &mut mesh_file_references,
                mesh_part.material.clone(),
            );
        }

        let mut scanned_importables = Vec::default();
        scanned_importables.push(ScannedImportable {
            name: None,
            asset_type: mesh_adv_asset_type,
            file_references: mesh_file_references,
        });

        scanned_importables
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
        let bytes = std::fs::read(path).unwrap();

        let b3f_reader = B3FReader::new(&bytes)
            .ok_or("Blender Mesh Import error, mesh file format not recognized")
            .unwrap();
        let mesh_as_json: HydrateMeshJson = serde_json::from_slice(b3f_reader.get_block(0))
            .map_err(|e| e.to_string())
            .unwrap();

        let mut import_data = MeshAdvMeshImportedDataRecord::new_single_object(schema_set).unwrap();
        let mut import_data_container =
            DataContainerMut::new_single_object(&mut import_data, schema_set);
        let x = MeshAdvMeshImportedDataRecord::default();

        //
        // Find the materials and assign them unique slot indexes
        //
        let mut material_slots = Vec::default();
        let mut material_slots_lookup = HashMap::default();
        for mesh_part in &mesh_as_json.mesh_parts {
            if !material_slots_lookup.contains_key(&mesh_part.material) {
                let slot_index = material_slots.len() as u32;
                material_slots.push(mesh_part.material.clone());
                material_slots_lookup.insert(mesh_part.material.clone(), slot_index);
            }
        }

        for mesh_part in &mesh_as_json.mesh_parts {
            //
            // Get byte slices of all input data for this mesh part
            //
            let positions_bytes = b3f_reader
                .get_block(mesh_part.position.ok_or("No position data").unwrap() as usize);
            let normals_bytes =
                b3f_reader.get_block(mesh_part.normal.ok_or("No normal data").unwrap() as usize);
            let tex_coords_bytes = b3f_reader.get_block(
                *mesh_part
                    .uv
                    .get(0)
                    .ok_or("No texture coordinate data")
                    .unwrap() as usize,
            );
            let part_indices_bytes = b3f_reader.get_block(mesh_part.indices as usize);

            //
            // Get strongly typed slices of all input data for this mesh part
            //

            // Indices may be encoded as u16 or u32, either way copy them out to a Vec<u32>
            let mut part_indices_u32 = Vec::<u32>::default();
            match mesh_part.index_type {
                MeshPartJsonIndexType::U16 => {
                    let part_indices_u16_ref = try_cast_u8_slice::<u16>(part_indices_bytes)
                        .ok_or("Could not cast due to alignment")
                        .unwrap();
                    part_indices_u32.reserve(part_indices_u16_ref.len());
                    for &part_index in part_indices_u16_ref {
                        part_indices_u32.push(part_index as u32);
                    }
                }
                MeshPartJsonIndexType::U32 => {
                    let part_indices_u32_ref = try_cast_u8_slice::<u32>(part_indices_bytes)
                        .ok_or("Could not cast due to alignment")
                        .unwrap();
                    part_indices_u32.reserve(part_indices_u32_ref.len());
                    for &part_index in part_indices_u32_ref {
                        part_indices_u32.push(part_index);
                    }
                }
            };

            let mut part_indices = PushBuffer::from_vec(&part_indices_u32).into_data();

            let material_index = *material_slots_lookup.get(&mesh_part.material).unwrap();

            let entry_uuid = x.mesh_parts().add_entry(&mut import_data_container);
            let entry = x.mesh_parts().entry(entry_uuid);
            entry
                .positions()
                .set(&mut import_data_container, positions_bytes.to_vec())
                .unwrap();
            entry
                .normals()
                .set(&mut import_data_container, normals_bytes.to_vec())
                .unwrap();
            entry
                .texture_coordinates()
                .set(&mut import_data_container, tex_coords_bytes.to_vec())
                .unwrap();
            entry
                .indices()
                .set(&mut import_data_container, part_indices)
                .unwrap();
            entry
                .material_index()
                .set(&mut import_data_container, material_index)
                .unwrap();
        }

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object =
                MeshAdvMeshAssetRecord::new_single_object(schema_set).unwrap();
            let mut default_asset_data_container =
                DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            let x = MeshAdvMeshAssetRecord::default();

            //
            // Set up the material slots
            //
            for material_slot in material_slots {
                let object_id = importable_objects
                    .get(&None)
                    .unwrap()
                    .referenced_paths
                    .get(&material_slot)
                    .unwrap();
                let entry = x
                    .material_slots()
                    .add_entry(&mut default_asset_data_container);
                x.material_slots()
                    .entry(entry)
                    .set(&mut default_asset_data_container, *object_id)
                    .unwrap();
            }

            default_asset_object
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

pub struct BlenderMeshAssetPlugin;

impl AssetPlugin for BlenderMeshAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<BlenderMeshImporter>(schema_linker);
    }
}
