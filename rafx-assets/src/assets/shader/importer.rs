use crate::assets::shader::ShaderAssetData;
use crate::schema::{
    GpuImageAssetRecord, GpuImageImportedDataRecord, ShaderPackageAssetRecord,
    ShaderPackageImportedDataRecord,
};
use distill::core::AssetUuid;
use distill::importer::{ImportOp, ImportedAsset, Importer, ImporterValue};
use hydrate_base::hashing::HashMap;
use hydrate_base::ObjectId;
use hydrate_data::{
    DataContainer, DataContainerMut, DataSet, Field, PropertyPath, Record, SchemaSet, SingleObject,
};
use hydrate_model::{
    job_system, AssetPlugin, Builder, BuilderRegistryBuilder, ImportableObject, ImportedImportable,
    ImporterRegistryBuilder, JobApi, JobEnumeratedDependencies, JobInput, JobOutput, JobProcessor,
    JobProcessorRegistryBuilder, ScannedImportable, SchemaLinker,
};
use image::GenericImageView;
use rafx_api::{RafxHashedShaderPackage, RafxShaderPackage, RafxShaderPackageVulkan};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use type_uuid::*;

// There may be a better way to do this type coercing
// fn coerce_result_str<T>(result: Result<T, &str>) -> distill::importer::Result<T> {
//     let ok = result.map_err(|x| -> Box<dyn std::error::Error + Send> { Box::<dyn std::error::Error + Send + Sync>::from(x) })?;
//     Ok(ok)
// }

fn coerce_result_string<T>(result: Result<T, String>) -> distill::importer::Result<T> {
    let ok = result.map_err(|x| -> Box<dyn std::error::Error + Send> {
        Box::<dyn std::error::Error + Send + Sync>::from(x)
    })?;
    Ok(ok)
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "867bc278-67b5-469c-aeea-1c05da722918"]
pub struct ShaderImporterSpvState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "90fdad4b-cec1-4f59-b679-97895711b6e1"]
pub struct ShaderImporterSpv;
impl Importer for ShaderImporterSpv {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        5
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = ShaderImporterSpvState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let asset_id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ShaderImporterSpvState(Some(asset_id));

        // Raw compiled shader
        let mut spv_bytes = Vec::new();
        source.read_to_end(&mut spv_bytes)?;

        log::trace!(
            "Import shader asset {:?} with {} bytes of code",
            asset_id,
            spv_bytes.len()
        );

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

        let shader_asset = ShaderAssetData {
            shader_package: hashed_shader_package,
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: asset_id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader_asset),
            }],
        })
    }
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "d4fb07ce-76e6-497e-ac31-bcaeb43528aa"]
pub struct ShaderImporterCookedState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "cab0cf4c-16ff-4dbd-aae7-8705246d85d6"]
pub struct ShaderImporterCooked;
impl Importer for ShaderImporterCooked {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        5
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = ShaderImporterCookedState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let asset_id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = ShaderImporterCookedState(Some(asset_id));

        // Raw compiled shader
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let hashed_shader_package: RafxHashedShaderPackage = coerce_result_string(
            bincode::deserialize::<RafxHashedShaderPackage>(&bytes)
                .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x)),
        )?;

        log::trace!(
            "Import shader asset {:?} with hash {:?}",
            asset_id,
            hashed_shader_package.shader_package_hash(),
        );

        let shader_asset = ShaderAssetData {
            shader_package: hashed_shader_package,
        };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: asset_id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader_asset),
            }],
        })
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "f0070e09-088b-4387-ba65-075657023733"]
pub struct ShaderPackageImporterSpv;

impl hydrate_model::Importer for ShaderPackageImporterSpv {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["spv"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
    ) -> Vec<ScannedImportable> {
        let asset_type = schema_set
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
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let spv_bytes = std::fs::read(path).unwrap();

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
                ShaderPackageImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::new_single_object(&mut import_object, schema_set);
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
            let mut default_asset_object =
                ShaderPackageAssetRecord::new_single_object(schema_set).unwrap();
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

impl hydrate_model::Importer for ShaderPackageImporterCooked {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["cookedshaderpackage"]
    }

    fn scan_file(
        &self,
        path: &Path,
        schema_set: &SchemaSet,
    ) -> Vec<ScannedImportable> {
        let asset_type = schema_set
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
        path: &Path,
        importable_objects: &HashMap<Option<String>, ImportableObject>,
        schema_set: &SchemaSet,
    ) -> HashMap<Option<String>, ImportedImportable> {
        //
        // Read the file
        //
        let cooked_shader_bytes = std::fs::read(path).unwrap();

        let hashed_shader_package: RafxHashedShaderPackage = coerce_result_string(
            bincode::deserialize::<RafxHashedShaderPackage>(&cooked_shader_bytes)
                .map_err(|x| format!("Failed to deserialize cooked shader: {:?}", x)),
        )
        .unwrap();

        log::trace!(
            "Import shader asset {:?} with hash {:?}",
            path,
            hashed_shader_package.shader_package_hash(),
        );

        let package_bytes = bincode::serialize(&hashed_shader_package).unwrap();

        //
        // Create import data
        //
        let import_data = {
            let mut import_object =
                ShaderPackageImportedDataRecord::new_single_object(schema_set).unwrap();
            let mut import_data_container =
                DataContainerMut::new_single_object(&mut import_object, schema_set);
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
            let mut default_asset_object =
                ShaderPackageAssetRecord::new_single_object(schema_set).unwrap();
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
    pub asset_id: ObjectId,
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
        data_set: &DataSet,
        schema_set: &SchemaSet,
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
        data_set: &DataSet,
        schema_set: &SchemaSet,
        dependency_data: &HashMap<ObjectId, SingleObject>,
        job_api: &dyn JobApi,
    ) -> ShaderPackageJobOutput {
        //
        // Read imported data
        //
        let imported_data = &dependency_data[&input.asset_id];
        let data_container = DataContainer::new_single_object(&imported_data, schema_set);
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
        asset_id: ObjectId,
        data_set: &DataSet,
        schema_set: &SchemaSet,
        job_api: &dyn JobApi,
    ) {
        //let data_container = DataContainer::new_dataset(data_set, schema_set, asset_id);
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
        schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        builder_registry: &mut BuilderRegistryBuilder,
        job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<ShaderPackageImporterSpv>(schema_linker);
        importer_registry.register_handler::<ShaderPackageImporterCooked>(schema_linker);
        builder_registry.register_handler::<ShaderPackageBuilder>(schema_linker);
        job_processor_registry.register_job_processor::<ShaderPackageJobProcessor>();
    }
}
