use crate::assets::shader::ShaderAssetData;
use crate::schema::{
    ShaderPackageAssetAccessor, ShaderPackageAssetOwned, ShaderPackageImportedDataOwned,
    ShaderPackageImportedDataReader,
};
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{RecordAccessor, RecordOwned};
use hydrate_pipeline::{
    AssetPlugin, Builder, BuilderContext, BuilderRegistryBuilder, EnumerateDependenciesContext,
    ImportContext, ImportedImportable, Importer, ImporterRegistryBuilder,
    JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    PipelineResult, RunContext, ScanContext, ScannedImportable, SchemaLinker,
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
    ) -> PipelineResult<Vec<ScannedImportable>> {
        let asset_type = context
            .schema_set
            .find_named_type(ShaderPackageAssetAccessor::schema_name())?
            .as_record()?
            .clone();
        Ok(vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
        }])
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<HashMap<Option<String>, ImportedImportable>> {
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
        let import_data = ShaderPackageImportedDataOwned::new_builder(context.schema_set);
        import_data.bytes().set(package_bytes)?;

        //
        // Create the default asset
        //
        let default_asset = ShaderPackageAssetOwned::new_builder(context.schema_set);

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data.into_inner()?),
                default_asset: Some(default_asset.into_inner()?),
            },
        );
        Ok(imported_objects)
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
    ) -> PipelineResult<Vec<ScannedImportable>> {
        let asset_type = context
            .schema_set
            .find_named_type(ShaderPackageAssetAccessor::schema_name())?
            .as_record()?
            .clone();
        Ok(vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
        }])
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<HashMap<Option<String>, ImportedImportable>> {
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
        let import_data = ShaderPackageImportedDataOwned::new_builder(context.schema_set);
        import_data.bytes().set(package_bytes)?;

        //
        // Create the default asset
        //
        let default_asset = ShaderPackageAssetOwned::new_builder(context.schema_set);

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data.into_inner()?),
                default_asset: Some(default_asset.into_inner()?),
            },
        );
        Ok(imported_objects)
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

    fn enumerate_dependencies(
        &self,
        context: EnumerateDependenciesContext<Self::InputT>,
    ) -> PipelineResult<JobEnumeratedDependencies> {
        // No dependencies
        Ok(JobEnumeratedDependencies {
            import_data: vec![context.input.asset_id],
            upstream_jobs: Vec::default(),
        })
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> PipelineResult<ShaderPackageJobOutput> {
        //
        // Read imported data
        //
        let imported_data =
            context.imported_data::<ShaderPackageImportedDataReader>(context.input.asset_id)?;
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
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, asset_id);
        //let x = ShaderPackageAssetAccessor::default();

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
