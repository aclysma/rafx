use crate::assets::shader::ShaderAssetData;
use crate::schema::{
    ShaderPackageAssetAccessor, ShaderPackageAssetRecord, ShaderPackageImportedDataRecord,
};
use hydrate_base::AssetId;
use hydrate_data::{Record, RecordAccessor};
use hydrate_pipeline::{
    AssetPlugin, Builder, BuilderContext, BuilderRegistryBuilder, ImportContext, Importer,
    ImporterRegistryBuilder, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    PipelineResult, RunContext, ScanContext, SchemaLinker,
};
use rafx_api::{RafxHashedShaderPackage, RafxShaderPackage, RafxShaderPackageVulkan};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "f0070e09-088b-4387-ba65-075657023733"]
pub struct ShaderPackageImporterSpv;

impl Importer for ShaderPackageImporterSpv {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["spv"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        context.add_default_importable::<ShaderPackageAssetRecord>()?;
        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let spv_bytes = std::fs::read(context.path)?;

        // The hash is used in some places identify the shader
        let shader_package = RafxShaderPackage {
            dx12: None,
            metal: None,
            vk: Some(RafxShaderPackageVulkan::SpvBytes(spv_bytes)),
            gles2: None,
            gles3: None,
            vk_reflection: None,
            dx12_reflection: None,
            metal_reflection: None,
            gles2_reflection: None,
            gles3_reflection: None,
            debug_name: None,
        };

        let hashed_shader_package = RafxHashedShaderPackage::new(shader_package);

        let package_bytes = Arc::new(bincode::serialize(&hashed_shader_package)?);

        //
        // Create import data
        //
        let import_data = ShaderPackageImportedDataRecord::new_builder(context.schema_set);
        import_data.bytes().set(package_bytes)?;

        //
        // Create the default asset
        //
        let default_asset = ShaderPackageAssetRecord::new_builder(context.schema_set);

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "ac37987a-6c92-41b1-ba46-a5cf575dee9f"]
pub struct ShaderPackageImporterCooked;

impl Importer for ShaderPackageImporterCooked {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["cookedshaderpackage"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        context.add_default_importable::<ShaderPackageAssetRecord>()?;
        Ok(())
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let cooked_shader_bytes = std::fs::read(context.path)?;

        let hashed_shader_package: RafxHashedShaderPackage =
            bincode::deserialize::<RafxHashedShaderPackage>(&cooked_shader_bytes)
                .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x))?;

        log::trace!(
            "Import shader asset {:?} with hash {:?}",
            context.path,
            hashed_shader_package.shader_package_hash(),
        );

        let package_bytes = Arc::new(bincode::serialize(&hashed_shader_package)?);

        //
        // Create import data
        //
        let import_data = ShaderPackageImportedDataRecord::new_builder(context.schema_set);
        import_data.bytes().set(package_bytes)?;

        //
        // Create the default asset
        //
        let default_asset = ShaderPackageAssetRecord::new_builder(context.schema_set);

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct ShaderPackageJobInput {
    pub asset_id: AssetId,
}
impl JobInput for ShaderPackageJobInput {}

#[derive(Serialize, Deserialize)]
pub struct ShaderPackageJobOutput {}
impl JobOutput for ShaderPackageJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "88998a4b-9216-4d01-a16d-ca1bff1c7c30"]
pub struct ShaderPackageJobProcessor;

impl JobProcessor for ShaderPackageJobProcessor {
    type InputT = ShaderPackageJobInput;
    type OutputT = ShaderPackageJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn run<'a>(
        &self,
        context: &'a RunContext<'a, Self::InputT>,
    ) -> PipelineResult<ShaderPackageJobOutput> {
        //
        // Read imported data
        //
        let imported_data =
            context.imported_data::<ShaderPackageImportedDataRecord>(context.input.asset_id)?;
        let shader_package = bincode::deserialize(&imported_data.bytes().get()?)?;

        //TODO: We can generate assets for different platforms

        //
        // Create the processed data
        //
        let processed_data = ShaderAssetData { shader_package };

        //
        // Serialize and return
        //
        context.produce_default_artifact(context.input.asset_id, processed_data)?;

        Ok(ShaderPackageJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "da6760e7-5b24-43b4-830d-6ee4515096b8"]
pub struct ShaderPackageBuilder {}

impl Builder for ShaderPackageBuilder {
    fn asset_type(&self) -> &'static str {
        ShaderPackageAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //Future: Might produce jobs per-platform
        context.enqueue_job::<ShaderPackageJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            ShaderPackageJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}

pub struct ShaderPackageAssetPlugin;

impl AssetPlugin for ShaderPackageAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<ShaderPackageImporterSpv>();
        importer_registry.register_handler::<ShaderPackageImporterCooked>();
        builder_registry.register_handler::<ShaderPackageBuilder>();
        job_processor_registry.register_job_processor::<ShaderPackageJobProcessor>();
    }
}
