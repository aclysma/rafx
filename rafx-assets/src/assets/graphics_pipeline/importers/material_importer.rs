use crate::assets::graphics_pipeline::{
    GraphicsPipelineShaderStage, MaterialAssetData, MaterialRon,
};
use crate::schema::{GraphicsPipelineShaderStageRef, MaterialAssetAccessor, MaterialAssetRecord};
use crate::MaterialPassData;
use hydrate_base::AssetId;
use hydrate_data::{DataSetResult, ImportableName, Record, RecordAccessor};
use hydrate_pipeline::{
    Builder, BuilderContext, HandleFactory, ImportContext, Importer, JobInput, JobOutput,
    JobProcessor, PipelineResult, RunContext, ScanContext,
};
use rafx_framework::MaterialShaderStage;
use serde::{Deserialize, Serialize};
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "64d8deb9-5aa5-48e6-9110-9b356e2bce3b"]
pub struct MaterialImporter;

impl Importer for MaterialImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["material"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let material_ron =
            ron::de::from_str::<MaterialRon>(&source).map_err(|e| format!("RON error {:?}", e))?;

        let importable = context.add_default_importable::<MaterialAssetRecord>()?;
        for pass in material_ron.passes {
            for stage in pass.shaders {
                importable.add_path_reference(&stage.shader_module)?;
            }
        }

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
        let material_ron =
            ron::de::from_str::<MaterialRon>(&source).map_err(|e| format!("RON error {:?}", e))?;

        //
        // Create the default asset
        //
        let default_asset = MaterialAssetRecord::new_builder(context.schema_set);
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
                stage
                    .shader_module()
                    .set(context.asset_id_for_referenced_file_path(
                        ImportableName::default(),
                        &stage_ron.shader_module.into(),
                    )?)?;
            }
        }

        context.add_default_importable(default_asset.into_inner()?, None);
        Ok(())
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

    fn run<'a>(
        &self,
        context: &'a RunContext<'a, Self::InputT>,
    ) -> PipelineResult<MaterialJobOutput> {
        //
        // Read asset data
        //
        let asset_data = context.asset::<MaterialAssetRecord>(context.input.asset_id)?;
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
                        record: GraphicsPipelineShaderStageRef,
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
