use crate::assets::compute_pipeline::{ComputePipelineAssetData, ComputePipelineRon};
use crate::assets::shader::ShaderPackageImporterCooked;
use crate::schema::ComputePipelineAssetAccessor;
use hydrate_base::AssetId;
use hydrate_data::{
    DataContainerRef, DataContainerRefMut, DataSet, HashMap, ImporterId, RecordAccessor, SchemaSet,
    SingleObject,
};
use hydrate_pipeline::{
    job_system, BuilderContext, BuilderRegistryBuilder, EnumerateDependenciesContext,
    ImportContext, ImportableAsset, ImportedImportable, ImporterRegistry, ImporterRegistryBuilder,
    JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    ReferencedSourceFile, RunContext, ScanContext, ScannedImportable, SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use type_uuid::*;
use uuid::Uuid;

#[derive(TypeUuid, Default)]
#[uuid = "a78c8ec9-11bf-45aa-886b-0080f3a52b40"]
pub struct HydrateComputePipelineImporter;

impl hydrate_pipeline::Importer for HydrateComputePipelineImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["compute"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path).unwrap();
        let parsed_source = ron::de::from_str::<ComputePipelineRon>(&source).unwrap();

        let asset_type = context
            .schema_set
            .find_named_type(ComputePipelineAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let shader_package_importer_id =
            ImporterId(Uuid::from_bytes(ShaderPackageImporterCooked::UUID));
        file_references.push(ReferencedSourceFile {
            importer_id: shader_package_importer_id,
            path: parsed_source.shader_module,
        });
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references,
        }]
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path).unwrap();
        let compute_pipeline_asset_data = ron::de::from_str::<ComputePipelineRon>(&source).unwrap();

        let shader_object_id = *context
            .importable_assets
            .get(&None)
            .unwrap()
            .referenced_paths
            .get(&compute_pipeline_asset_data.shader_module)
            .unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object =
                ComputePipelineAssetAccessor::new_single_object(context.schema_set).unwrap();
            let mut default_asset_data_container = DataContainerRefMut::from_single_object(
                &mut default_asset_object,
                context.schema_set,
            );
            let x = ComputePipelineAssetAccessor::default();

            x.entry_name()
                .set(
                    &mut default_asset_data_container,
                    compute_pipeline_asset_data.entry_name,
                )
                .unwrap();
            x.shader_module()
                .set(&mut default_asset_data_container, shader_object_id)
                .unwrap();

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

    fn enumerate_dependencies(
        &self,
        context: EnumerateDependenciesContext<Self::InputT>,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies::default()
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> ComputePipelineJobOutput {
        //
        // Read asset data
        //
        let data_container = DataContainerRef::from_dataset(
            context.data_set,
            context.schema_set,
            context.input.asset_id,
        );
        let x = ComputePipelineAssetAccessor::default();

        let shader_module = x.shader_module().get(data_container).unwrap();
        let entry_name = x.entry_name().get(data_container).unwrap();

        context.produce_default_artifact_with_handles(context.input.asset_id, |handle_factory| {
            let shader_module = handle_factory.make_handle_to_default_artifact(shader_module);
            ComputePipelineAssetData {
                entry_name,
                shader_module,
            }
        });

        ComputePipelineJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "d3e81f20-1f66-4e65-b542-1861a15b24b6"]
pub struct ComputePipelineBuilder {}

impl hydrate_pipeline::Builder for ComputePipelineBuilder {
    fn asset_type(&self) -> &'static str {
        ComputePipelineAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) {
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, asset_id);
        //let x = ComputePipelineAssetAccessor::default();

        //Future: Might produce jobs per-platform
        context.enqueue_job::<ComputePipelineJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            ComputePipelineJobInput {
                asset_id: context.asset_id,
            },
        );
    }
}

pub struct ComputePipelineAssetPlugin;

impl hydrate_pipeline::AssetPlugin for ComputePipelineAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<HydrateComputePipelineImporter>();
        builder_registry.register_handler::<ComputePipelineBuilder>();
        job_processor_registry.register_job_processor::<ComputePipelineJobProcessor>();
    }
}
