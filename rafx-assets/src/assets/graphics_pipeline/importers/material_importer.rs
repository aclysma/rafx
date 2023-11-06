use crate::assets::graphics_pipeline::{
    GraphicsPipelineShaderStage, MaterialAssetData, MaterialInstanceAssetData, MaterialRon,
    SamplerAssetData,
};
use crate::assets::shader::ShaderPackageImporterCooked;
use crate::schema::{GraphicsPipelineShaderStageRecord, MaterialAssetRecord};
use crate::MaterialPassData;
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, ImporterId, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableObject, ImportedImportable, ImporterRegistry,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable, SchemaLinker,
};
use rafx_framework::MaterialShaderStage;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use type_uuid::*;
use uuid::Uuid;

#[derive(TypeUuid, Default)]
#[uuid = "64d8deb9-5aa5-48e6-9110-9b356e2bce3b"]
pub struct HydrateMaterialImporter;

impl hydrate_model::Importer for HydrateMaterialImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["material"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
        importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(path).unwrap();
        let material_ron = ron::de::from_str::<MaterialRon>(&source).unwrap();

        let asset_type = schema_set
            .find_named_type(MaterialAssetRecord::schema_name())
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
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(path).unwrap();
        let material_ron = ron::de::from_str::<MaterialRon>(&source).unwrap();
        println!("Importing material {:?}", material_ron);

        // let shader_object_id = *importable_objects
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
                MaterialAssetRecord::new_single_object(schema_set).unwrap();
            let mut default_asset_data_container =
                DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            let x = MaterialAssetRecord::default();

            for pass in material_ron.passes {
                let entry = x.passes().add_entry(&mut default_asset_data_container);
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
                            *importable_objects
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
    pub asset_id: ObjectId,
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
        input: &MaterialJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies::default()
    }

    fn run(
        &self,
        input: &MaterialJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> MaterialJobOutput {
        //
        // Read asset data
        //
        let data_container = DataContainer::new_dataset(data_set, schema_set, input.asset_id);
        let x = MaterialAssetRecord::default();

        job_system::produce_asset_with_handles(job_api, input.asset_id, || {
            //let shader_module = job_system::make_handle_to_default_artifact(job_api, shader_module);
            let mut passes = Vec::default();
            for pass_entry in x.passes().resolve_entries(&data_container).into_iter() {
                let pass_entry = x.passes().entry(*pass_entry);

                let fixed_function_state = ron::de::from_str(
                    &pass_entry
                        .fixed_function_state()
                        .get(&data_container)
                        .unwrap(),
                )
                .unwrap();

                fn read_stage(
                    stage: MaterialShaderStage,
                    record: &GraphicsPipelineShaderStageRecord,
                    data_container: &DataContainer,
                    job_api: &dyn JobApi,
                    stages: &mut Vec<GraphicsPipelineShaderStage>,
                ) {
                    let entry_name = record.entry_name().get(&data_container).unwrap();
                    let shader_module = record.shader_module().get(&data_container).unwrap();

                    if entry_name.is_empty() && shader_module.is_null() {
                        return;
                    }

                    stages.push(GraphicsPipelineShaderStage {
                        stage,
                        shader_module: job_system::make_handle_to_default_artifact(
                            job_api,
                            shader_module,
                        ),
                        entry_name,
                    });
                }

                let mut shaders = Vec::default();
                read_stage(
                    MaterialShaderStage::Vertex,
                    &pass_entry.vertex_stage(),
                    &data_container,
                    job_api,
                    &mut shaders,
                );
                read_stage(
                    MaterialShaderStage::Fragment,
                    &pass_entry.fragment_stage(),
                    &data_container,
                    job_api,
                    &mut shaders,
                );

                let name = pass_entry.name().get(&data_container).unwrap();
                let phase = pass_entry.phase().get(&data_container).unwrap();

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

impl hydrate_model::Builder for MaterialBuilder {
    fn asset_type(&self) -> &'static str {
        MaterialAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::new_dataset(data_set, schema_set, asset_id);
        //let x = MaterialAssetRecord::default();

        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<MaterialJobProcessor>(
            data_set,
            schema_set,
            job_api,
            MaterialJobInput { asset_id },
        );
    }
}
