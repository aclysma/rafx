use crate::assets::mesh_adv::{
    BlenderModelImporter, ModelAdvAsset, PrefabAdvAssetDataObjectLightKind,
};
use crate::schema::{MeshAdvPrefabAssetAccessor, MeshAdvPrefabImportDataAccessor};
use hydrate_base::handle::Handle;
use hydrate_base::hashing::HashMap;
use hydrate_data::{DataContainerRefMut, ImporterId, RecordAccessor};
use hydrate_pipeline::{
    AssetPlugin, BuilderRegistryBuilder, ImportContext, ImportedImportable, Importer,
    ImporterRegistryBuilder, JobProcessorRegistryBuilder, PipelineResult, ReferencedSourceFile,
    ScanContext, ScannedImportable, SchemaLinker,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use type_uuid::*;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectTransform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectModel {
    pub model: Handle<ModelAdvAsset>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum MeshAdvPrefabJsonFormatObjectLightKind {
    Point,
    Spot,
    Directional,
}

impl Into<PrefabAdvAssetDataObjectLightKind> for MeshAdvPrefabJsonFormatObjectLightKind {
    fn into(self) -> PrefabAdvAssetDataObjectLightKind {
        match self {
            MeshAdvPrefabJsonFormatObjectLightKind::Point => {
                PrefabAdvAssetDataObjectLightKind::Point
            }
            MeshAdvPrefabJsonFormatObjectLightKind::Spot => PrefabAdvAssetDataObjectLightKind::Spot,
            MeshAdvPrefabJsonFormatObjectLightKind::Directional => {
                PrefabAdvAssetDataObjectLightKind::Directional
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectLightSpot {
    pub inner_angle: f32,
    pub outer_angle: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectLight {
    pub color: [f32; 3],
    pub kind: MeshAdvPrefabJsonFormatObjectLightKind,
    pub intensity: f32,
    pub cutoff_distance: Option<f32>,
    pub spot: Option<MeshAdvPrefabJsonFormatObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObject {
    pub transform: MeshAdvPrefabJsonFormatObjectTransform,
    pub model: Option<MeshAdvPrefabJsonFormatObjectModel>,
    pub light: Option<MeshAdvPrefabJsonFormatObjectLight>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormat {
    pub objects: Vec<MeshAdvPrefabJsonFormatObject>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HydrateMeshAdvPrefabJsonFormatObjectModel {
    pub model: PathBuf, //Handle<ModelAdvAsset>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HydrateMeshAdvPrefabJsonFormatObject {
    pub transform: MeshAdvPrefabJsonFormatObjectTransform,
    pub model: Option<HydrateMeshAdvPrefabJsonFormatObjectModel>,
    pub light: Option<MeshAdvPrefabJsonFormatObjectLight>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HydrateMeshAdvPrefabJsonFormat {
    pub objects: Vec<HydrateMeshAdvPrefabJsonFormatObject>,
}

#[derive(TypeUuid, Default)]
#[uuid = "a40a442f-285e-4bb8-81f4-43d761b9f140"]
pub struct BlenderPrefabImporter;

impl Importer for BlenderPrefabImporter {
    fn supported_file_extensions(&self) -> &[&'static str] {
        &["blender_prefab"]
    }

    fn scan_file(
        &self,
        context: ScanContext,
    ) -> PipelineResult<Vec<ScannedImportable>> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path).unwrap();
        let json_format: HydrateMeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))
            .unwrap();

        let asset_type = context
            .schema_set
            .find_named_type(MeshAdvPrefabAssetAccessor::schema_name())
            .unwrap()
            .as_record()
            .unwrap()
            .clone();

        let mut file_references: Vec<ReferencedSourceFile> = Default::default();
        let model_importer_id = ImporterId(Uuid::from_bytes(BlenderModelImporter::UUID));

        for object in &json_format.objects {
            if let Some(model) = &object.model {
                file_references.push(ReferencedSourceFile {
                    importer_id: model_importer_id,
                    path: model.model.clone(),
                })
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
        let source = std::fs::read_to_string(context.path).unwrap();
        // We don't actually need to parse this now but worth doing to make sure it's well-formed at import time
        let _json_format: HydrateMeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))
            .unwrap();

        //
        // Create the default asset
        //
        let default_asset = {
            let default_asset_object =
                MeshAdvPrefabAssetAccessor::new_single_object(context.schema_set).unwrap();
            // let mut default_asset_data_container =
            //     DataContainerRefMut::from_single_object(&mut default_asset_object, schema_set);
            // let x = MeshAdvPrefabAssetAccessor::default();

            // No fields to write
            default_asset_object
        };

        let import_data = {
            let mut import_data_object =
                MeshAdvPrefabImportDataAccessor::new_single_object(context.schema_set).unwrap();
            let mut import_data_data_container = DataContainerRefMut::from_single_object(
                &mut import_data_object,
                context.schema_set,
            );
            let x = MeshAdvPrefabImportDataAccessor::default();

            x.json_data()
                .set(&mut import_data_data_container, source)
                .unwrap();

            // No fields to write
            import_data_object
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
        Ok(imported_objects)
    }
}

pub struct BlenderPrefabAssetPlugin;

impl AssetPlugin for BlenderPrefabAssetPlugin {
    fn setup(
        _schema_linker: &mut SchemaLinker,
        importer_registry: &mut ImporterRegistryBuilder,
        _builder_registry: &mut BuilderRegistryBuilder,
        _job_processor_registry: &mut JobProcessorRegistryBuilder,
    ) {
        importer_registry.register_handler::<BlenderPrefabImporter>();
    }
}
