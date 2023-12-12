use crate::assets::graphics_pipeline::{MaterialInstanceAssetData, MaterialInstanceRon};
use crate::schema::{MaterialInstanceAssetAccessor, MaterialInstanceAssetRecord};
use crate::MaterialInstanceSlotAssignment;
use hydrate_base::AssetId;
use hydrate_data::{ImportableName, NullOverride, Record, RecordAccessor};
use hydrate_pipeline::{
    Builder, BuilderContext, ImportContext, Importer, JobInput, JobOutput, JobProcessor,
    PipelineResult, RunContext, ScanContext,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "c5936989-35dc-432c-80ee-30842c172774"]
pub struct MaterialInstanceImporter;
//
// impl HydrateMaterialInstanceImporter {
//     pub fn set_material_instance_properties(
//         meterial_instance: &MaterialInstanceRon
//     ) {
//
//     }
// }

impl Importer for MaterialInstanceImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["materialinstance"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let material_instance_ron = ron::de::from_str::<MaterialInstanceRon>(&source)
            .map_err(|e| format!("RON error {:?}", e))?;

        let importable = context.add_default_importable::<MaterialInstanceAssetRecord>()?;

        importable.add_path_reference(&material_instance_ron.material)?;

        for pass in material_instance_ron.slot_assignments {
            if let Some(image) = &pass.image {
                importable.add_path_reference(image)?;
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
        let material_ron = ron::de::from_str::<MaterialInstanceRon>(&source)
            .map_err(|e| format!("RON error {:?}", e))?;

        //
        // Create the default asset
        //
        let default_asset = MaterialInstanceAssetRecord::new_builder(context.schema_set);
        for slot_assignment in material_ron.slot_assignments {
            let entry_uuid = default_asset.slot_assignments().add_entry()?;
            let entry = default_asset.slot_assignments().entry(entry_uuid);

            entry.slot_name().set(slot_assignment.slot_name)?;
            entry
                .array_index()
                .set(slot_assignment.array_index as u32)?;

            if let Some(image) = &slot_assignment.image {
                let image_asset_id = context
                    .asset_id_for_referenced_file_path(ImportableName::default(), &image.into())?;
                entry.image().set(image_asset_id)?;
            }

            if let Some(sampler) = &slot_assignment.sampler {
                let sampler_ron =
                    ron::ser::to_string(sampler).map_err(|e| format!("RON error {:?}", e))?;
                entry.sampler().set(sampler_ron)?;
            }

            if let Some(buffer_data) = slot_assignment.buffer_data {
                entry
                    .buffer_data()
                    .set_null_override(NullOverride::SetNonNull)?
                    .expect("We set a field to be non-null but couldn't unwrap it")
                    .set(Arc::new(buffer_data))?;
            }
        }

        let material_asset_id = context.asset_id_for_referenced_file_path(
            ImportableName::default(),
            &material_ron.material.into(),
        )?;
        default_asset.material().set(material_asset_id)?;

        //
        // Return the created objects
        //
        context.add_default_importable(default_asset.into_inner()?, None);
        Ok(())
    }
}

#[derive(Hash, Serialize, Deserialize)]
pub struct MaterialInstanceJobInput {
    pub asset_id: AssetId,
}
impl JobInput for MaterialInstanceJobInput {}

#[derive(Serialize, Deserialize)]
pub struct MaterialInstanceJobOutput {}
impl JobOutput for MaterialInstanceJobOutput {}

#[derive(Default, TypeUuid)]
#[uuid = "1d32096c-edf2-4662-91a2-5e5513de0979"]
pub struct MaterialInstanceJobProcessor;

impl JobProcessor for MaterialInstanceJobProcessor {
    type InputT = MaterialInstanceJobInput;
    type OutputT = MaterialInstanceJobOutput;

    fn version(&self) -> u32 {
        1
    }

    fn run<'a>(
        &self,
        context: &'a RunContext<'a, Self::InputT>,
    ) -> PipelineResult<MaterialInstanceJobOutput> {
        //
        // Read asset data
        //
        let asset_data = context.asset::<MaterialInstanceAssetRecord>(context.input.asset_id)?;

        context.produce_default_artifact_with_handles(
            context.input.asset_id,
            |handle_factory| {
                let material =
                    handle_factory.make_handle_to_default_artifact(asset_data.material().get()?);

                let mut slot_assignments = Vec::default();
                for slot_assignent_entry in
                    asset_data.slot_assignments().resolve_entries()?.into_iter()
                {
                    let slot_assignment =
                        asset_data.slot_assignments().entry(*slot_assignent_entry);

                    let slot_name = slot_assignment.slot_name().get()?;
                    let array_index = slot_assignment.array_index().get()? as usize;

                    let image_object_id = slot_assignment.image().get()?;
                    let image = if image_object_id.is_null() {
                        None
                    } else {
                        Some(handle_factory.make_handle_to_default_artifact(image_object_id))
                    };

                    let sampler_ron = slot_assignment.sampler().get()?;
                    let sampler = if sampler_ron.is_empty() {
                        None
                    } else {
                        let sampler = ron::de::from_str(&slot_assignment.sampler().get()?)
                            .map_err(|e| format!("RON error {:?}", e))?;
                        Some(sampler)
                    };

                    let buffer_data =
                        if let Some(buffer_data) = slot_assignment.buffer_data().resolve_null()? {
                            Some(buffer_data.get()?.clone())
                        } else {
                            None
                        };

                    slot_assignments.push(MaterialInstanceSlotAssignment {
                        slot_name: (*slot_name).clone(),
                        array_index,
                        image,
                        sampler,
                        buffer_data: buffer_data.map(|x| (*x).clone()),
                    });
                }

                Ok(MaterialInstanceAssetData {
                    slot_assignments,
                    material,
                })
            },
        )?;

        Ok(MaterialInstanceJobOutput {})
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "0cfe8812-b0cd-4b72-bfa2-8ac3d30af7dd"]
pub struct MaterialInstanceBuilder {}

impl Builder for MaterialInstanceBuilder {
    fn asset_type(&self) -> &'static str {
        MaterialInstanceAssetAccessor::schema_name()
    }

    fn start_jobs(
        &self,
        context: BuilderContext,
    ) -> PipelineResult<()> {
        //Future: Might produce jobs per-platform
        context.enqueue_job::<MaterialInstanceJobProcessor>(
            context.data_set,
            context.schema_set,
            context.job_api,
            MaterialInstanceJobInput {
                asset_id: context.asset_id,
            },
        )?;
        Ok(())
    }
}
