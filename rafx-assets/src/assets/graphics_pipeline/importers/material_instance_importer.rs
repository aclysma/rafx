use crate::assets::graphics_pipeline::material_importer::HydrateMaterialImporter;
use crate::assets::graphics_pipeline::{MaterialInstanceAssetData, MaterialInstanceRon};
use crate::schema::MaterialInstanceAssetRecord;
use crate::{GpuImageImporterSimple, MaterialInstanceSlotAssignment};
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, ImporterId, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, ImportableObject, ImportedImportable, ImporterRegistry, JobApi,
    JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor, ReferencedSourceFile,
    ScannedImportable,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
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

impl hydrate_model::Importer for HydrateMaterialInstanceImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["materialinstance"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
        _importer_registry: &ImporterRegistry,
    ) -> Vec<ScannedImportable> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(path).unwrap();
        let material_instance_ron = ron::de::from_str::<MaterialInstanceRon>(&source).unwrap();

        let asset_type = schema_set
            .find_named_type(MaterialInstanceAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
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
        let material_ron = ron::de::from_str::<MaterialInstanceRon>(&source).unwrap();

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
                MaterialInstanceAssetRecord::new_single_object(schema_set).unwrap();
            let mut default_asset_data_container =
                DataContainerMut::new_single_object(&mut default_asset_object, schema_set);
            let x = MaterialInstanceAssetRecord::default();

            for slot_assignment in material_ron.slot_assignments {
                let entry_uuid = x
                    .slot_assignments()
                    .add_entry(&mut default_asset_data_container);
                let entry = x.slot_assignments().entry(entry_uuid);

                entry
                    .slot_name()
                    .set(&mut default_asset_data_container, slot_assignment.slot_name)
                    .unwrap();
                entry
                    .array_index()
                    .set(
                        &mut default_asset_data_container,
                        slot_assignment.array_index as u32,
                    )
                    .unwrap();

                if let Some(image) = &slot_assignment.image {
                    let image = *importable_objects
                        .get(&None)
                        .unwrap()
                        .referenced_paths
                        .get(image)
                        .unwrap();
                    entry
                        .image()
                        .set(&mut default_asset_data_container, image)
                        .unwrap();
                }

                if let Some(sampler) = &slot_assignment.sampler {
                    let sampler_ron = ron::ser::to_string(sampler).unwrap();
                    entry
                        .sampler()
                        .set(&mut default_asset_data_container, sampler_ron)
                        .unwrap();
                }

                if let Some(buffer_data) = slot_assignment.buffer_data {
                    entry
                        .buffer_data()
                        .set_not_null(&mut default_asset_data_container)
                        .set(&mut default_asset_data_container, buffer_data)
                        .unwrap();
                }
            }

            let material = *importable_objects
                .get(&None)
                .unwrap()
                .referenced_paths
                .get(&material_ron.material)
                .unwrap();
            x.material()
                .set(&mut default_asset_data_container, material)
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
pub struct MaterialInstanceJobInput {
    pub asset_id: ObjectId,
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
        _input: &MaterialInstanceJobInput,
        _data_set: &DataSet,
        _schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies::default()
    }

    fn run(
        &self,
        input: &MaterialInstanceJobInput,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        _dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> MaterialInstanceJobOutput {
        //
        // Read asset data
        //
        let data_container = DataContainer::new_dataset(data_set, schema_set, input.asset_id);
        let x = MaterialInstanceAssetRecord::default();

        job_system::produce_asset_with_handles(job_api, input.asset_id, || {
            let material = job_system::make_handle_to_default_artifact(
                job_api,
                x.material().get(&data_container).unwrap(),
            );

            let mut slot_assignments = Vec::default();
            for slot_assignent_entry in x
                .slot_assignments()
                .resolve_entries(&data_container)
                .into_iter()
            {
                let slot_assignment = x.slot_assignments().entry(*slot_assignent_entry);

                let slot_name = slot_assignment.slot_name().get(&data_container).unwrap();
                let array_index =
                    slot_assignment.array_index().get(&data_container).unwrap() as usize;

                let image_object_id = slot_assignment.image().get(&data_container).unwrap();
                let image = if image_object_id.is_null() {
                    None
                } else {
                    Some(job_system::make_handle_to_default_artifact(
                        job_api,
                        image_object_id,
                    ))
                };

                let sampler_ron = slot_assignment.sampler().get(&data_container).unwrap();
                let sampler = if sampler_ron.is_empty() {
                    None
                } else {
                    let sampler =
                        ron::de::from_str(&slot_assignment.sampler().get(&data_container).unwrap())
                            .unwrap();
                    Some(sampler)
                };

                let buffer_data = if let Some(buffer_data) =
                    slot_assignment.buffer_data().resolve_null(&data_container)
                {
                    Some(buffer_data.get(&data_container).unwrap().clone())
                } else {
                    None
                };

                slot_assignments.push(MaterialInstanceSlotAssignment {
                    slot_name,
                    array_index,
                    image,
                    sampler,
                    buffer_data,
                });
            }

            MaterialInstanceAssetData {
                slot_assignments,
                material,
            }
        });

        MaterialInstanceJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "0cfe8812-b0cd-4b72-bfa2-8ac3d30af7dd"]
pub struct MaterialInstanceBuilder {}

impl hydrate_model::Builder for MaterialInstanceBuilder {
    fn asset_type(&self) -> &'static str {
        MaterialInstanceAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::new_dataset(data_set, schema_set, asset_id);
        //let x = MaterialInstanceAssetRecord::default();

        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<MaterialInstanceJobProcessor>(
            data_set,
            schema_set,
            job_api,
            MaterialInstanceJobInput { asset_id },
        );
    }
}
