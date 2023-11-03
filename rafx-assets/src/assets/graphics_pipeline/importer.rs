use crate::assets::graphics_pipeline::{
    GraphicsPipelineShaderStage, HydrateGraphicsPipelineShaderStage, HydrateMaterialAssetData,
    HydrateMaterialPassData, MaterialAssetData, MaterialInstanceAssetData, MaterialRon,
    SamplerAssetData,
};
use crate::assets::shader::ShaderPackageImporterCooked;
use crate::schema::{GraphicsPipelineShaderStageRecord, MaterialAssetRecord};
use crate::MaterialPassData;
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, ImporterId, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ReferencedSourceFile, ScannedImportable, SchemaLinker,
};
use rafx_framework::MaterialShaderStage;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use type_uuid::*;
use uuid::Uuid;

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "62e662dc-cb15-444f-a7ac-eb89f52a4042"]
pub struct SamplerImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "9dfad44f-72e8-4ba6-b89a-96b017fb9cd9"]
pub struct SamplerImporter;
impl Importer for SamplerImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        2
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = SamplerImporterState;

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
        *state = SamplerImporterState(Some(id));

        let sampler_asset = ron::de::from_reader::<_, SamplerAssetData>(source)?;
        log::trace!("IMPORTED SAMPLER:\n{:#?}", sampler_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(sampler_asset),
            }],
        })
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "5cfac411-55a1-49dc-b07e-1ac486f9fe98"]
pub struct MaterialImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "eb9a20b7-3957-46fd-b832-2e7e99852bb0"]
pub struct MaterialImporter;
impl Importer for MaterialImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        2
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MaterialImporterState;

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
        *state = MaterialImporterState(Some(id));

        let material_asset = ron::de::from_reader::<_, MaterialAssetData>(source)?;
        log::trace!("IMPORTED MATERIAL:\n{:#?}", material_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(material_asset),
            }],
        })
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "d40e33f3-ba7d-4218-8266-a18d7c65b06e"]
pub struct MaterialInstanceImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ce02143-a5c4-4433-b843-07cdccf013b0"]
pub struct MaterialInstanceImporter;
impl Importer for MaterialInstanceImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        6
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MaterialInstanceImporterState;

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
        *state = MaterialInstanceImporterState(Some(id));

        let material_asset = ron::de::from_reader::<_, MaterialInstanceAssetData>(source)?;
        log::trace!("IMPORTED MATERIALINSTANCE:\n{:#?}", material_asset);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(material_asset),
            }],
        })
    }
}

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
#[uuid = "f4f71f13-e075-46cf-b601-bf233ec491a9"]
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
                    stages: &mut Vec<HydrateGraphicsPipelineShaderStage>,
                ) {
                    let entry_name = record.entry_name().get(&data_container).unwrap();
                    let shader_module = record.shader_module().get(&data_container).unwrap();

                    if entry_name.is_empty() && shader_module.is_null() {
                        return;
                    }

                    HydrateGraphicsPipelineShaderStage {
                        stage,
                        shader_module: job_system::make_handle_to_default_artifact(
                            job_api,
                            shader_module,
                        ),
                        entry_name,
                    };
                }

                let mut shaders = Vec::default();
                read_stage(
                    MaterialShaderStage::Vertex,
                    &pass_entry.vertex_stage(),
                    &data_container,
                    job_api,
                    &mut shaders,
                );

                passes.push(HydrateMaterialPassData {
                    name: Some(pass_entry.name().get(&data_container).unwrap()),
                    phase: Some(pass_entry.phase().get(&data_container).unwrap()),
                    fixed_function_state,
                    shaders,
                });
            }

            HydrateMaterialAssetData { passes }
        });

        job_system::produce_asset_with_handles(job_api, input.asset_id, || {
            //let shader_module = job_system::make_handle_to_default_artifact(job_api, shader_module);
            HydrateMaterialAssetData {
                passes: Default::default(),
            }
        });

        MaterialJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "5ae26801-6bad-480c-93ba-af1edb60df34"]
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

pub struct MaterialAssetPlugin;

impl hydrate_model::AssetPlugin for MaterialAssetPlugin {
    fn setup(
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<HydrateMaterialImporter>(schema_linker);
        builder_registry.register_handler::<MaterialBuilder>(schema_linker);
        job_processor_registry.register_job_processor::<MaterialJobProcessor>();
    }
}
