use crate::assets::graphics_pipeline::material_importer::HydrateMaterialImporter;
use crate::assets::graphics_pipeline::{MaterialInstanceAssetData, MaterialInstanceRon};
use crate::schema::{
    MaterialInstanceAssetAccessor, MaterialInstanceAssetOwned, MaterialInstanceAssetReader,
};
use crate::{GpuImageImporterSimple, MaterialInstanceSlotAssignment};
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{ImporterId, NullOverride, RecordAccessor, RecordOwned};
use hydrate_pipeline::{
    Builder, BuilderContext, EnumerateDependenciesContext, ImportContext, ImportedImportable,
    Importer, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor, PipelineResult,
    ReferencedSourceFile, RunContext, ScanContext, ScannedImportable,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;
use uuid::Uuid;

#[derive(TypeUuid, Default)]
#[uuid = "c5936989-35dc-432c-80ee-30842c172774"]
pub struct HydrateMaterialInstanceImporter;
//
// impl HydrateMaterialInstanceImporter {
//     pub fn set_material_instance_properties(
//         meterial_instance: &MaterialInstanceRon
//     ) {
//
//     }
// }

impl Importer for HydrateMaterialInstanceImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["materialinstance"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<Vec<ScannedImportable>> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let material_instance_ron = ron::de::from_str::<MaterialInstanceRon>(&source)
            .map_err(|e| format!("RON error {:?}", e))?;

        let asset_type = context
            .schema_set
            .find_named_type(MaterialInstanceAssetAccessor::schema_name())?
            .as_record()?
            .clone();
        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let image_importer_id = ImporterId(Uuid::from_bytes(GpuImageImporterSimple::UUID));
        let material_importer_id = ImporterId(Uuid::from_bytes(HydrateMaterialImporter::UUID));

        file_references.push(ReferencedSourceFile {
            importer_id: material_importer_id,
            path: material_instance_ron.material.clone(),
        });

        for pass in material_instance_ron.slot_assignments {
            if let Some(image) = &pass.image {
                file_references.push(ReferencedSourceFile {
                    importer_id: image_importer_id,
                    path: image.clone(),
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
        let material_ron = ron::de::from_str::<MaterialInstanceRon>(&source)
            .map_err(|e| format!("RON error {:?}", e))?;

        //
        // Create the default asset
        //
        let default_asset = MaterialInstanceAssetOwned::new_builder(context.schema_set);
        for slot_assignment in material_ron.slot_assignments {
            let entry_uuid = default_asset.slot_assignments().add_entry()?;
            let entry = default_asset.slot_assignments().entry(entry_uuid);

            entry.slot_name().set(slot_assignment.slot_name)?;
            entry
                .array_index()
                .set(slot_assignment.array_index as u32)?;

            if let Some(image) = &slot_assignment.image {
                let image = *context
                    .importable_assets
                    .get(&None)
                    .ok_or("Could not find default importable in importable_assets")?
                    .referenced_paths
                    .get(image)
                    .ok_or("Could not find asset ID associated with path")?;
                entry.image().set(image)?;
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

        let material = *context
            .importable_assets
            .get(&None)
            .ok_or("Could not find default importable in importable_assets")?
            .referenced_paths
            .get(&material_ron.material)
            .ok_or("Could not find asset ID associated with path")?;
        default_asset.material().set(material)?;

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
    ) -> PipelineResult<MaterialInstanceJobOutput> {
        //
        // Read asset data
        //
        let asset_data = context.asset::<MaterialInstanceAssetReader>(context.input.asset_id)?;

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
        //let data_container = DataContainerRef::from_dataset(data_set, schema_set, asset_id);
        //let x = MaterialInstanceAssetAccessor::default();

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
