use crate::assets::graphics_pipeline::{
    GraphicsPipelineShaderStage, MaterialAssetData, MaterialRon,
};
use crate::assets::shader::ShaderPackageImporterCooked;
use crate::schema::{
    GraphicsPipelineShaderStageReader, MaterialAssetAccessor, MaterialAssetOwned,
    MaterialAssetReader,
};
use crate::MaterialPassData;
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{DataSetResult, ImporterId, RecordAccessor, RecordOwned};
use hydrate_pipeline::{
    Builder, BuilderContext, EnumerateDependenciesContext, HandleFactory, ImportContext,
    ImportedImportable, Importer, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    PipelineResult, ReferencedSourceFile, RunContext, ScanContext, ScannedImportable,
};
use rafx_framework::MaterialShaderStage;
use serde::{Deserialize, Serialize};
use type_uuid::*;
use uuid::Uuid;

#[derive(TypeUuid, Default)]
#[uuid = "64d8deb9-5aa5-48e6-9110-9b356e2bce3b"]
pub struct HydrateMaterialImporter;

impl Importer for HydrateMaterialImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["material"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<Vec<ScannedImportable>> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let material_ron =
            ron::de::from_str::<MaterialRon>(&source).map_err(|e| format!("RON error {:?}", e))?;

        let asset_type = context
            .schema_set
            .find_named_type(MaterialAssetAccessor::schema_name())?
            .as_record()?
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
        Ok(vec![ScannedImportable {
            name: None,
            asset_type,
            file_references,
        }])
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> PipelineResult<HashMap<Option<String>, ImportedImportable>> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let material_ron =
            ron::de::from_str::<MaterialRon>(&source).map_err(|e| format!("RON error {:?}", e))?;

        //
        // Create the default asset
        //
        let default_asset = MaterialAssetOwned::new_builder(context.schema_set);
        for pass_ron in material_ron.passes {
            let entry = default_asset.passes().add_entry()?;
            let pass = default_asset.passes().entry(entry);
            pass.name().set(pass_ron.name.unwrap_or_default())?;
            pass.phase().set(pass_ron.phase.unwrap_or_default())?;

            let fixed_function_state = ron::ser::to_string(&pass_ron.fixed_function_state)
                .map_err(|e| format!("RON error {:?}", e))?;

            pass.fixed_function_state().set(fixed_function_state)?;
            for stage_ron in pass_ron.shaders {
                let stage = match stage_ron.stage {
                    MaterialShaderStage::Vertex => pass.vertex_stage(),
                    MaterialShaderStage::TessellationControl => unimplemented!(),
                    MaterialShaderStage::TessellationEvaluation => unimplemented!(),
                    MaterialShaderStage::Geometry => unimplemented!(),
                    MaterialShaderStage::Fragment => pass.fragment_stage(),
                    MaterialShaderStage::Compute => unimplemented!(),
                };

                stage.entry_name().set(stage_ron.entry_name)?;
                stage.shader_module().set(
                    *context
                        .importable_assets
                        .get(&None)
                        .ok_or("Could not find default importable in importable_assets")?
                        .referenced_paths
                        .get(&stage_ron.shader_module)
                        .ok_or("Could not find asset ID associated with path")?,
                )?;
            }
        }

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: None,
                default_asset: Some(default_asset.into_inner()?),
            },
        );
        Ok(imported_objects)
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
        _context: EnumerateDependenciesContext<Self::InputT>,
    ) -> PipelineResult<JobEnumeratedDependencies> {
        // No dependencies
        Ok(JobEnumeratedDependencies::default())
    }

    fn run(
        &self,
        context: RunContext<Self::InputT>,
    ) -> PipelineResult<MaterialJobOutput> {
        //
        // Read asset data
        //
        let asset_data = context.asset::<MaterialAssetReader>(context.input.asset_id)?;
        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                //let shader_module = job_system::make_handle_to_default_artifact(job_api, shader_module);
                let mut passes = Vec::default();
                for pass_entry in asset_data.passes().resolve_entries()?.into_iter() {
                    let pass_entry = asset_data.passes().entry(*pass_entry);

                    let fixed_function_state =
                        ron::de::from_str(&pass_entry.fixed_function_state().get()?)
                            .map_err(|e| format!("RON error {:?}", e))?;

                    fn read_stage(
                        stage: MaterialShaderStage,
                        record: GraphicsPipelineShaderStageReader,
                        stages: &mut Vec<GraphicsPipelineShaderStage>,
                        handle_factory: HandleFactory,
                    ) -> DataSetResult<()> {
                        let entry_name = record.entry_name().get()?;
                        let shader_module = record.shader_module().get()?;

                        if entry_name.is_empty() && shader_module.is_null() {
                            return Ok(());
                        }

                        stages.push(GraphicsPipelineShaderStage {
                            stage,
                            shader_module: handle_factory
                                .make_handle_to_default_artifact(shader_module),
                            entry_name: (*entry_name).clone(),
                        });
                        Ok(())
                    }

                    let mut shaders = Vec::default();
                    read_stage(
                        MaterialShaderStage::Vertex,
                        pass_entry.vertex_stage(),
                        &mut shaders,
                        handle_factory,
                    )?;
                    read_stage(
                        MaterialShaderStage::Fragment,
                        pass_entry.fragment_stage(),
                        &mut shaders,
                        handle_factory,
                    )?;

                    let name = pass_entry.name().get()?;
                    let phase = pass_entry.phase().get()?;

                    passes.push(MaterialPassData {
                        name: (!name.is_empty()).then(|| (*name).clone()),
                        phase: (!phase.is_empty()).then(|| (*phase).clone()),
                        fixed_function_state,
                        shaders,
                    });
                }

                Ok(MaterialAssetData { passes })
            },
        )?;

        Ok(MaterialJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "8fe3e85c-2adf-4424-9197-9391c2c8f3ce"]
pub struct MaterialBuilder {}

impl Builder for MaterialBuilder {
    fn asset_type(&self) -> &'static str {
        MaterialAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
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
        )?;
        Ok(())
    }
}
