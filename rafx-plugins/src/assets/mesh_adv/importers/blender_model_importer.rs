use crate::assets::mesh_adv::{BlenderMeshImporter, MeshAdvAsset};
use crate::schema::MeshAdvModelAssetRecord;
use hydrate_base::handle::Handle;
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerMut, ImporterId, Record, SchemaSet};
use hydrate_model::{
    BuilderRegistryBuilder, ImportableObject, ImportedImportable, ImporterRegistry,
    ImporterRegistryBuilder, JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable,
    SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use type_uuid::*;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct ModelLodJsonFormat {
    pub mesh: Handle<MeshAdvAsset>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelJsonFormat {
    pub lods: Vec<ModelLodJsonFormat>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HydrateModelLodJsonFormat {
    pub mesh: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct HydrateModelJsonFormat {
    pub lods: Vec<HydrateModelLodJsonFormat>,
}

#[derive(TypeUuid, Default)]
#[uuid = "a97c46e9-1deb-4ca2-9f70-b4ce97a74cf2"]
pub struct BlenderModelImporter;

impl hydrate_model::Importer for BlenderModelImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_model"]
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
        let json_format: HydrateModelJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Model Import error: {:?}", x))
            .unwrap();

        let asset_type = schema_set
            .find_named_type(MeshAdvModelAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let shader_package_importer_id = ImporterId(Uuid::from_bytes(BlenderMeshImporter::UUID));
        for lod in &json_format.lods {
            file_references.push(ReferencedSourceFile {
                importer_id: shader_package_importer_id,
                path: lod.mesh.clone(),
            });
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
        let json_format: HydrateModelJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Model Import error: {:?}", x))
            .unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object =
                MeshAdvModelAssetRecord::new_single_object(schema_set).unwrap();
            let mut default_asset_data_container =
                DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            let x = MeshAdvModelAssetRecord::default();

            let entry = x.lods().add_entry(&mut default_asset_data_container);
            let lod_entry = x.lods().entry(entry);

            for lod in &json_format.lods {
                let mesh_object_id = *importable_objects
                    .get(&None)
                    .unwrap()
                    .referenced_paths
                    .get(&lod.mesh)
                    .unwrap();

                lod_entry
                    .mesh()
                    .set(&mut default_asset_data_container, mesh_object_id)
                    .unwrap();
            }

            // No fields to write
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
                import_data: None,
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}

pub struct BlenderModelAssetPlugin;

impl hydrate_model::AssetPlugin for BlenderModelAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        _builder_registry: &mut BuilderRegistryBuilder,
        _job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<BlenderModelImporter>();
    }
}
