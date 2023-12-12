use crate::assets::compute_pipeline::{ComputePipelineAssetData, ComputePipelineRon};
use crate::schema::{ComputePipelineAssetAccessor, ComputePipelineAssetRecord};
use hydrate_base::AssetId;
use hydrate_data::{ImportableName, Record, RecordAccessor};
use hydrate_pipeline::{
    AssetPlugin, Builder, BuilderContext, BuilderRegistryBuilder, ImportContext, Importer,
    ImporterRegistryBuilder, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    PipelineResult, RunContext, ScanContext,
};
use serde::{Deserialize, Serialize};
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "a78c8ec9-11bf-45aa-886b-0080f3a52b40"]
pub struct ComputePipelineImporter;

impl Importer for ComputePipelineImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["compute"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let parsed_source = ron::de::from_str::<ComputePipelineRon>(&source)
            .map_err(|e| format!("RON error {:?}", e))?;

        let importable = context.add_default_importable::<ComputePipelineAssetRecord>()?;
        importable.add_path_reference(parsed_source.shader_module)?;
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
        let compute_pipeline_asset_data = ron::de::from_str::<ComputePipelineRon>(&source)
            .map_err(|e| format!("RON error {:?}", e))?;

        let shader_object_id = context.asset_id_for_referenced_file_path(
            ImportableName::default(),
            &compute_pipeline_asset_data.shader_module.into(),
        )?;

        //
        // Create the default asset
        //
        let default_asset = ComputePipelineAssetRecord::new_builder(context.schema_set);
        default_asset
            .entry_name()
            .set(compute_pipeline_asset_data.entry_name)?;
        default_asset.shader_module().set(shader_object_id)?;

        //
        // Return the created objects
        //
        context.add_default_importable(default_asset.into_inner()?, None);
        Ok(())
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct ComputePipelineJobInput {
    pub asset_id: AssetId,
}
impl JobInput for ComputePipelineJobInput {}

#[derive(Serialize, Deserialize)]
pub struct ComputePipelineJobOutput {}
impl JobOutput for ComputePipelineJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "b0af85fe-54ed-4e02-a707-589e916fef74"]
pub struct ComputePipelineJobProcessor;

impl JobProcessor for ComputePipelineJobProcessor {
    type InputT = ComputePipelineJobInput;
    type OutputT = ComputePipelineJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn run<'a>(
        &self,
        context: &'a RunContext<'a, Self::InputT>,
    ) -> PipelineResult<ComputePipelineJobOutput> {
        //
        // Read asset data
        //
        let asset_data = context.asset::<ComputePipelineAssetRecord>(context.input.asset_id)?;

        let shader_module = asset_data.shader_module().get()?;
        let entry_name = asset_data.entry_name().get()?;

        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                let shader_module = handle_factory.make_handle_to_default_artifact(shader_module);
                Ok(ComputePipelineAssetData {
                    entry_name: (*entry_name).clone(),
                    shader_module,
                })
            },
        )?;

        Ok(ComputePipelineJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "d3e81f20-1f66-4e65-b542-1861a15b24b6"]
pub struct ComputePipelineBuilder {}

impl Builder for ComputePipelineBuilder {
    fn asset_type(&self) -> &'static str {
        ComputePipelineAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //Future: Might produce jobs per-platform
        context.enqueue_job::<ComputePipelineJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            ComputePipelineJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

pub struct ComputePipelineAssetPlugin;

impl AssetPlugin for ComputePipelineAssetPlugin {
    fn setup(
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<ComputePipelineImporter>();
        builder_registry.register_handler::<ComputePipelineBuilder>();
        job_processor_registry.register_job_processor::<ComputePipelineJobProcessor>();
    }
}
