use crate::assets::font::FontAssetData;
use crate::schema::{FontAssetAccessor, FontAssetRecord, FontImportedDataRecord};
use fnv::FnvHasher;
use hydrate_base::AssetId;
use hydrate_data::{Record, RecordAccessor};
use hydrate_pipeline::{
    AssetPlugin, Builder, BuilderContext, BuilderRegistryBuilder, ImportContext, Importer,
    ImporterRegistryBuilder, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    PipelineResult, RunContext, ScanContext, SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "b99453db-4d59-4801-8b89-c86ba6fb4620"]
pub struct FontImporter;

impl Importer for FontImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["ttf"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        context.add_default_importable::<FontAssetRecord>()?;
        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let font_bytes = std::fs::read(context.path)?;

        //
        // Create the default asset
        //
        let default_asset = FontAssetRecord::new_builder(context.schema_set);

        //
        // Create import data
        //
        let import_data = FontImportedDataRecord::new_builder(context.schema_set);
        import_data.bytes().set(Arc::new(font_bytes))?;

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
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

    fn run<'a>(
        &self,
        context: &'a RunContext<'a, Self::InputT>,
    ) -> PipelineResult<FontJobOutput> {
        //
        // Read asset properties
        //

        //
        // Read imported data
        //
        let imported_data =
            context.imported_data::<FontImportedDataRecord>(context.input.asset_id)?;

        let font_bytes = imported_data.bytes().get()?.clone();

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
            data: (*font_bytes).clone(),
            scale: scale as f32,
        };

        //
        // Serialize and return
        //
        context.produce_default_artifact(context.input.asset_id, processed_data)?;

        Ok(FontJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "834e2100-00b6-4d7b-8fbd-196ee8b998f1"]
pub struct FontBuilder {}

impl Builder for FontBuilder {
    fn asset_type(&self) -> &'static str {
        FontAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //Future: Might produce jobs per-platform
        context.enqueue_job::<FontJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            FontJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

pub struct FontAssetPlugin;

impl AssetPlugin for FontAssetPlugin {
    fn setup(
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<FontImporter>();
        builder_registry.register_handler::<FontBuilder>();
        job_processor_registry.register_job_processor::<FontJobProcessor>();
    }
}
