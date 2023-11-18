use crate::assets::font::FontAssetData;
use crate::schema::{FontAssetAccessor, FontImportedDataAccessor};
use fnv::FnvHasher;
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{
    DataContainerRef, DataContainerRefMut, DataSet, FieldAccessor, PropertyPath, RecordAccessor,
    SchemaSet, SingleObject,
};
use hydrate_pipeline::{
    job_system, BuilderContext, BuilderRegistryBuilder, EnumerateDependenciesContext,
    ImportContext, ImportableAsset, ImportedImportable, ImporterRegistry, ImporterRegistryBuilder,
    JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    RunContext, ScanContext, ScannedImportable, SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::Path;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "b99453db-4d59-4801-8b89-c86ba6fb4620"]
pub struct HydrateFontImporter;

impl hydrate_pipeline::Importer for HydrateFontImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["ttf"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        let asset_type = context
            .schema_set
            .find_named_type(FontAssetAccessor::schema_name())
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
        context: ImportContext,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let font_bytes = std::fs::read(context.path).unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object =
                FontAssetAccessor::new_single_object(context.schema_set).unwrap();
            // let mut default_asset_data_container =
            //     DataContainerRefMut::from_single_object(&mut default_asset_object, schema_set);
            // let x = FontAssetAccessor::default();

            // No fields to write
            default_asset_object
        };

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                FontImportedDataAccessor::new_single_object(context.schema_set).unwrap();
            let mut import_data_container =
                DataContainerRefMut::from_single_object(&mut import_object, context.schema_set);
            let x = FontImportedDataAccessor::default();
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
        context: EnumerateDependenciesContext<Self::InputT>,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies {
            import_data: vec![context.input.asset_id],
            upstream_jobs: Default::default(),
        }
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> FontJobOutput {
        //
        // Read asset properties
        //
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, input.asset_id);
        //let x = FontAssetAccessor::default();

        //
        // Read imported data
        //
        let imported_data = &context.dependency_data[&context.input.asset_id];
        let data_container =
            DataContainerRef::from_single_object(&imported_data, context.schema_set);
        let x = FontImportedDataAccessor::new(PropertyPath::default());

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
        context.produce_default_artifact(context.input.asset_id, processed_data);

        FontJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "834e2100-00b6-4d7b-8fbd-196ee8b998f1"]
pub struct FontBuilder {}

impl hydrate_pipeline::Builder for FontBuilder {
    fn asset_type(&self) -> &'static str {
        FontAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) {
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, asset_id);
        //let x = FontAssetAccessor::default();

        //Future: Might produce jobs per-platform
        context.enqueue_job::<FontJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            FontJobInput {
                asset_id: context.asset_id,
            },
        );
    }
}

pub struct FontAssetPlugin;

impl hydrate_pipeline::AssetPlugin for FontAssetPlugin {
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
