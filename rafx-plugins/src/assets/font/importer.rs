use crate::assets::font::FontAssetData;
use crate::schema::{FontAssetRecord, FontImportedDataRecord};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use fnv::FnvHasher;
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, Field, PropertyPath, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ScannedImportable, SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::Path;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "c0228ccb-c3d6-40c1-aa19-458f93b5aff9"]
pub struct FontImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "51631327-a334-4191-9ff2-eab7106e1cae"]
pub struct FontImporter;
impl Importer for FontImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        3
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = FontImporterState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = FontImporterState(Some(id));
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let scale = 40;

        let mut hasher = FnvHasher::default();
        bytes.hash(&mut hasher);
        scale.hash(&mut hasher);
        let data_hash = hasher.finish();

        let asset_data = FontAssetData {
            data_hash,
            data: bytes,
            scale: scale as f32,
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(asset_data),
            }],
        })
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "b99453db-4d59-4801-8b89-c86ba6fb4620"]
pub struct HydrateFontImporter;

impl hydrate_model::Importer for HydrateFontImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["ttf"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
    ) -> Vec<ScannedImportable> {
        let asset_type = schema_set
            .find_named_type(FontAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
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
        let font_bytes = std::fs::read(path).unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object = FontAssetRecord::new_single_object(schema_set).unwrap();
            // let mut default_asset_data_container =
            //     DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            // let x = FontAssetRecord::default();

            // No fields to write
            default_asset_object
        };

        //
        // Create import data
        //
        let import_data = {
            let mut import_object = FontImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::new_single_object(&mut import_object, schema_set);
            let x = FontImportedDataRecord::default();
            x.bytes()
                .set(&mut import_data_container, font_bytes)
                .unwrap();
            import_object
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
pub struct FontJobInput {
    pub asset_id: ObjectId,
}
impl JobInput for FontJobInput {}

#[derive(Serialize, Deserialize)]
pub struct FontJobOutput {}
impl JobOutput for FontJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "b49738d7-e5f6-4144-8fc6-83018802ef94"]
pub struct FontJobProcessor;

impl JobProcessor for FontJobProcessor {
    type InputT = FontJobInput;
    type OutputT = FontJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn enumerate_dependencies(
        &self,
        input: &FontJobInput,
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
        input: &FontJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> FontJobOutput {
        //
        // Read asset properties
        //
        let data_container = DataContainer::new_dataset(data_set, schema_set, input.asset_id);
        let x = FontAssetRecord::default();

        //
        // Read imported data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::new_single_object(&imported_data, schema_set);
        let x = FontImportedDataRecord::new(PropertyPath::default());

        let font_bytes = x.bytes().get(&data_container).unwrap().clone();

        let scale = 40i32;

        let mut hasher = FnvHasher::default();
        font_bytes.hash(&mut hasher);
        scale.hash(&mut hasher);
        let data_hash = hasher.finish();

        //
        // Create the processed data
        //
        let processed_data = FontAssetData {
            data_hash,
            data: font_bytes,
            scale: scale as f32,
        };

        //
        // Serialize and return
        //
        job_system::produce_asset(job_api, input.asset_id, processed_data);

        FontJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "834e2100-00b6-4d7b-8fbd-196ee8b998f1"]
pub struct FontBuilder {}

impl hydrate_model::Builder for FontBuilder {
    fn asset_type(&self) -> &'static str {
        FontAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::new_dataset(data_set, schema_set, asset_id);
        //let x = FontAssetRecord::default();

        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<FontJobProcessor>(
            data_set,
            schema_set,
            job_api,
            FontJobInput { asset_id },
        );
    }
}

pub struct FontAssetPlugin;

impl hydrate_model::AssetPlugin for FontAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<HydrateFontImporter>(schema_linker);
        builder_registry.register_handler::<FontBuilder>(schema_linker);
        job_processor_registry.register_job_processor::<FontJobProcessor>();
    }
}
