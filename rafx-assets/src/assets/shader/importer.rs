use crate::assets::shader::ShaderAssetData;
use crate::schema::{ShaderPackageAssetRecord, ShaderPackageImportedDataRecord};
use hydrate_base::hashing::HashMap;
use hydrate_base::AssetId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, Field, PropertyPath, Record, SchemaSet, SingleObject,
};
use hydrate_pipeline::{
    job_system, AssetPlugin, Builder, BuilderRegistryBuilder, ImportContext, ImportableAsset,
    ImportedImportable, ImporterRegistry, ImporterRegistryBuilder, JobApi,
    JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor, JobProcessorRegistryBuilder,
    ScanContext, ScannedImportable, SchemaLinker,
};
use rafx_api::{RafxHashedShaderPackage, RafxShaderPackage, RafxShaderPackageVulkan};
use serde::{Deserialize, Serialize};
use std::path::Path;
use type_uuid::*;

#[derive(TypeUuid, Default)]
#[uuid = "f0070e09-088b-4387-ba65-075657023733"]
pub struct ShaderPackageImporterSpv;

impl hydrate_pipeline::Importer for ShaderPackageImporterSpv {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["spv"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        let asset_type = context
            .schema_set
            .find_named_type(ShaderPackageAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
        }]
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let spv_bytes = std::fs::read(context.path).unwrap();

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

        let package_bytes = bincode::serialize(&hashed_shader_package).unwrap();

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                ShaderPackageImportedDataRecord::new_single_object(context.schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::from_single_object(&mut import_object, context.schema_set);
            let x = ShaderPackageImportedDataRecord::default();
            x.bytes()
                .set(&mut import_data_container, package_bytes)
                .unwrap();
            import_object
        };

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object =
                ShaderPackageAssetRecord::new_single_object(context.schema_set).unwrap();
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
                import_data: Some(import_data),
                default_asset: Some(default_asset),
            },
        );
        imported_objects
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "ac37987a-6c92-41b1-ba46-a5cf575dee9f"]
pub struct ShaderPackageImporterCooked;

impl hydrate_pipeline::Importer for ShaderPackageImporterCooked {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["cookedshaderpackage"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> Vec<ScannedImportable> {
        let asset_type = context
            .schema_set
            .find_named_type(ShaderPackageAssetRecord::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();
        vec![ScannedImportable {
            name: None,
            asset_type,
            file_references: Default::default(),
        }]
    }

    fn import_file(
        &self,
        context: ImportContext,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let cooked_shader_bytes = std::fs::read(context.path).unwrap();

        let hashed_shader_package: RafxHashedShaderPackage =
            bincode::deserialize::<RafxHashedShaderPackage>(&cooked_shader_bytes)
                .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x))
                .unwrap();

        log::trace!(
            "Import shader asset {:?} with hash {:?}",
            context.path,
            hashed_shader_package.shader_package_hash(),
        );

        let package_bytes = bincode::serialize(&hashed_shader_package).unwrap();

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                ShaderPackageImportedDataRecord::new_single_object(context.schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::from_single_object(&mut import_object, context.schema_set);
            let x = ShaderPackageImportedDataRecord::default();
            x.bytes()
                .set(&mut import_data_container, package_bytes)
                .unwrap();
            import_object
        };

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object =
                ShaderPackageAssetRecord::new_single_object(context.schema_set).unwrap();
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
                import_data: Some(import_data),
                default_asset: Some(default_asset),
            },
        );
        imported_objects
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
        input: &ShaderPackageJobInput,
        _data_set: &DataSet,
        _schema_set: &SchemaSet,
    ) -> JobEnumeratedDependencies {
        // No dependencies
        JobEnumeratedDependencies {
            import_data: vec![input.asset_id],
            upstream_jobs: Vec::default(),
        }
    }

    fn run(
        &self,
        input: &ShaderPackageJobInput,
        _data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<AssetId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> ShaderPackageJobOutput {
        //
        // Read imported data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::from_single_object(&imported_data, schema_set);
        let x = ShaderPackageImportedDataRecord::new(PropertyPath::default());

        let shader_package =
            bincode::deserialize(&x.bytes().get(&data_container).unwrap()).unwrap();

        //TODO: We can generate assets for different platforms

        //
        // Create the processed data
        //
        let processed_data = ShaderAssetData { shader_package };

        //
        // Serialize and return
        //
        job_system::produce_asset(job_api, input.asset_id, processed_data);

        ShaderPackageJobOutput {}
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "da6760e7-5b24-43b4-830d-6ee4515096b8"]
pub struct ShaderPackageBuilder {}

impl Builder for ShaderPackageBuilder {
    fn asset_type(&self) -> &'static str {
        ShaderPackageAssetRecord::schema_name()
    }

    fn start_jobs(
        &self,
        asset_id: AssetId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::from_dataset(data_set, schema_set, asset_id);
        //let x = ShaderPackageAssetRecord::default();

        //Future: Might produce jobs per-platform
        job_system::enqueue_job::<ShaderPackageJobProcessor>(
            data_set,
            schema_set,
            job_api,
            ShaderPackageJobInput { asset_id },
        );
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
