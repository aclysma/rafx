use crate::assets::font::FontAssetData;
use crate::schema::{FontAssetRecord, FontImportedDataRecord};
use fnv::FnvHasher;
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, Field, PropertyPath, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableAsset, ImportedImportable, ImporterRegistry,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ScannedImportable, SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::Path;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "b99453db-4d59-4801-8b89-c86ba6fb4620"]
pub struct HydrateFontImporter;

impl hydrate_model::Importer for HydrateFontImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["ttf"]
    }

    fn scan_file(
        &self,
        _path: &Path,
        schema_set: &SchemaSet,
        _importer_registry: &ImporterRegistry,
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
        importable_assets: &HashMap<Option<String>, ImportableAsset>,
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
            let default_asset_object = FontAssetRecord::new_single_object(schema_set).unwrap();
            // let mut default_asset_data_container =
            //     DataContainerMut::from_single_object(&mut default_asset_object, schema_set);
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
                DataContainerMut::from_single_object(&mut import_object, schema_set);
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
    pub asset_id: AssetId,
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
        input: &FontJobInput,
        _data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<AssetId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> FontJobOutput {
        //
        // Read asset properties
        //
        //let data_container = DataContainer::from_dataset(data_set, schema_set, input.asset_id);
        //let x = FontAssetRecord::default();

        //
        // Read imported data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::from_single_object(&imported_data, schema_set);
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
        asset_id: AssetId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::from_dataset(data_set, schema_set, asset_id);
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
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<HydrateFontImporter>();
        builder_registry.register_handler::<FontBuilder>();
        job_processor_registry.register_job_processor::<FontJobProcessor>();
    }
}
