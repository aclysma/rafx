use crate::assets::graphics_pipeline::{
    GraphicsPipelineShaderStage, MaterialAssetData, MaterialRon,
};
use crate::assets::shader::ShaderPackageImporterCooked;
use crate::schema::{GraphicsPipelineShaderStageAccessor, MaterialAssetAccessor};
use crate::MaterialPassData;
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{
    DataContainerRef, DataContainerRefMut, DataSet, ImporterId, RecordAccessor, SchemaSet,
    SingleObject,
};
use hydrate_pipeline::{
    job_system, BuilderContext, EnumerateDependenciesContext, HandleFactory, ImportContext,
    ImportableAsset, ImportedImportable, ImporterRegistry, JobEnumeratedDependencies, JobInput,
    JobOutput, JobProcessor, ReferencedSourceFile, RunContext, ScanContext, ScannedImportable,
};
use rafx_framework::MaterialShaderStage;
use serde::{Deserialize, Serialize};
use std::path::Path;
use type_uuid::*;
use uuid::Uuid;

#[derive(TypeUuid, Default)]
#[uuid = "64d8deb9-5aa5-48e6-9110-9b356e2bce3b"]
pub struct HydrateMaterialImporter;

impl hydrate_pipeline::Importer for HydrateMaterialImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["material"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path).unwrap();
        let material_ron = ron::de::from_str::<MaterialRon>(&source).unwrap();

        let asset_type = context
            .schema_set
            .find_named_type(MaterialAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let shader_package_importer_id =
            ImporterId(Uuid::from_bytes(ShaderPackageImporterCooked::UUID));
        for pass in material_ron.passes {
            for stage in pass.shaders {
                file_references.push(ReferencedSourceFile {
                    importer_id: shader_package_importer_id,
                    path: stage.shader_module,
                });
            }
        }
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
        let material_ron = ron::de::from_str::<MaterialRon>(&source).unwrap();

        // let shader_object_id = *importable_assets
        //     .get(&None)
        //     .unwrap()
        //     .referenced_paths
        //     .get(&compute_pipeline_asset_data.shader_module)
        //     .unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let mut default_asset_object =
                MaterialAssetAccessor::new_single_object(context.schema_set).unwrap();
            let mut default_asset_data_container = DataContainerRefMut::from_single_object(
                &mut default_asset_object,
                context.schema_set,
            );
            let x = MaterialAssetAccessor::default();

            for pass in material_ron.passes {
                let entry = x
                    .passes()
                    .add_entry(&mut default_asset_data_container)
                    .unwrap();
                let x = x.passes().entry(entry);
                x.name()
                    .set(
                        &mut default_asset_data_container,
                        pass.name.unwrap_or_default(),
                    )
                    .unwrap();
                x.phase()
                    .set(
                        &mut default_asset_data_container,
                        pass.phase.unwrap_or_default(),
                    )
                    .unwrap();

                let fixed_function_state = ron::ser::to_string(&pass.fixed_function_state).unwrap();

                x.fixed_function_state()
                    .set(&mut default_asset_data_container, fixed_function_state)
                    .unwrap();
                for stage in pass.shaders {
                    let x = match stage.stage {
                        MaterialShaderStage::Vertex => x.vertex_stage(),
                        MaterialShaderStage::TessellationControl => unimplemented!(),
                        MaterialShaderStage::TessellationEvaluation => unimplemented!(),
                        MaterialShaderStage::Geometry => unimplemented!(),
                        MaterialShaderStage::Fragment => x.fragment_stage(),
                        MaterialShaderStage::Compute => unimplemented!(),
                    };

                    x.entry_name()
                        .set(&mut default_asset_data_container, stage.entry_name)
                        .unwrap();
                    x.shader_module()
                        .set(
                            &mut default_asset_data_container,
                            *context
                                .importable_assets
                                .get(&None)
                                .unwrap()
                                .referenced_paths
                                .get(&stage.shader_module)
                                .unwrap(),
                        )
                        .unwrap();
                }
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

#[derive(Hash, Serialize, Deserialize)]
pub struct MaterialJobInput {
    pub asset_id: AssetId,
}
impl JobInput for MaterialJobInput {}

#[derive(Serialize, Deserialize)]
pub struct MaterialJobOutput {}
impl JobOutput for MaterialJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "da22bde4-d702-41c1-8e6a-5d9c553020ef"]
pub struct MaterialJobProcessor;

impl JobProcessor for MaterialJobProcessor {
    type InputT = MaterialJobInput;
    type OutputT = MaterialJobOutput;

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
    ) -> MaterialJobOutput {
        //
        // Read asset data
        //
        let data_container = DataContainerRef::from_dataset(
            context.data_set,
            context.schema_set,
            context.input.asset_id,
        );
        let x = MaterialAssetAccessor::default();

        context.produce_default_artifact_with_handles(context.input.asset_id, |handle_factory| {
            //let shader_module = job_system::make_handle_to_default_artifact(job_api, shader_module);
            let mut passes = Vec::default();
            for pass_entry in x
                .passes()
                .resolve_entries(data_container)
                .unwrap()
                .into_iter()
            {
                let pass_entry = x.passes().entry(*pass_entry);

                let fixed_function_state = ron::de::from_str(
                    &pass_entry
                        .fixed_function_state()
                        .get(data_container)
                        .unwrap(),
                )
                .unwrap();

                fn read_stage(
                    stage: MaterialShaderStage,
                    record: &GraphicsPipelineShaderStageAccessor,
                    data_container: DataContainerRef,
                    stages: &mut Vec<GraphicsPipelineShaderStage>,
                    handle_factory: HandleFactory,
                ) {
                    let entry_name = record.entry_name().get(data_container).unwrap();
                    let shader_module = record.shader_module().get(data_container).unwrap();

                    if entry_name.is_empty() && shader_module.is_null() {
                        return;
                    }

                    stages.push(GraphicsPipelineShaderStage {
                        stage,
                        shader_module: handle_factory
                            .make_handle_to_default_artifact(shader_module),
                        entry_name,
                    });
                }

                let mut shaders = Vec::default();
                read_stage(
                    MaterialShaderStage::Vertex,
                    &pass_entry.vertex_stage(),
                    data_container,
                    &mut shaders,
                    handle_factory,
                );
                read_stage(
                    MaterialShaderStage::Fragment,
                    &pass_entry.fragment_stage(),
                    data_container,
                    &mut shaders,
                    handle_factory,
                );

                let name = pass_entry.name().get(data_container).unwrap();
                let phase = pass_entry.phase().get(data_container).unwrap();

                passes.push(MaterialPassData {
                    name: (!name.is_empty()).then(|| name),
                    phase: (!phase.is_empty()).then(|| phase),
                    fixed_function_state,
                    shaders,
                });
            }

            MaterialAssetData { passes }
        });

        MaterialJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "8fe3e85c-2adf-4424-9197-9391c2c8f3ce"]
pub struct MaterialBuilder {}

impl hydrate_pipeline::Builder for MaterialBuilder {
    fn asset_type(&self) -> &'static str {
        MaterialAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) {
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, asset_id);
        //let x = MaterialAssetAccessor::default();

        //Future: Might produce jobs per-platform
        context.enqueue_job::<MaterialJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            MaterialJobInput {
                asset_id: context.asset_id,
            },
        );
    }
}
