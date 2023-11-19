use crate::assets::mesh_adv::{BlenderMeshImporter, MeshAdvAsset};
use crate::schema::{MeshAdvModelAssetAccessor, MeshAdvModelAssetOwned};
use hydrate_base::handle::Handle;
use hydrate_base::hashing::HashMap;
use hydrate_data::{ImporterId, RecordAccessor, RecordOwned};
use hydrate_pipeline::{
    AssetPlugin, BuilderRegistryBuilder, ImportContext, ImportedImportable, Importer,
    ImporterRegistryBuilder, JobProcessorRegistryBuilder, PipelineResult, ReferencedSourceFile,
    ScanContext, ScannedImportable, SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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

impl Importer for BlenderModelImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_model"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<Vec<ScannedImportable>> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let json_format: HydrateModelJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Model Import error: {:?}", x))?;

        let asset_type = context
            .schema_set
            .find_named_type(MeshAdvModelAssetAccessor::schema_name())?
            .as_record()?
            .clone();
        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let shader_package_importer_id = ImporterId(Uuid::from_bytes(BlenderMeshImporter::UUID));
        for lod in &json_format.lods {
            file_references.push(ReferencedSourceFile {
                importer_id: shader_package_importer_id,
                path: lod.mesh.clone(),
            });
        }
        Ok(vec![ScannedImportable {
            name: None,
            asset_type,
            file_references,
        }])
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<HashMap<Option<String>, ImportedImportable>> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let json_format: HydrateModelJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Model Import error: {:?}", x))?;

        //
        // Create the default asset
        //
        let default_asset = MeshAdvModelAssetOwned::new_builder(context.schema_set);

        let entry = default_asset.lods().add_entry()?;
        let lod_entry = default_asset.lods().entry(entry);

        for lod in &json_format.lods {
            let mesh_object_id = *context
                .importable_assets
                .get(&None)
                .ok_or("Could not find default importable in importable_assets")?
                .referenced_paths
                .get(&lod.mesh)
                .ok_or("Could not find asset ID associated with path")?;

            lod_entry.mesh().set(mesh_object_id)?;
        }

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: None,
                default_asset: Some(default_asset.into_inner()?),
            },
        );
        Ok(imported_objects)
    }
}

pub struct BlenderModelAssetPlugin;

impl AssetPlugin for BlenderModelAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        _builder_registry: &mut BuilderRegistryBuilder,
        _job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<BlenderModelImporter>();
    }
}
