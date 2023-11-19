use crate::assets::mesh_adv::{
    BlenderModelImporter, ModelAdvAsset, PrefabAdvAssetDataObjectLightKind,
};
use crate::schema::{
    MeshAdvPrefabAssetAccessor, MeshAdvPrefabAssetOwned, MeshAdvPrefabImportDataOwned,
};
use hydrate_base::handle::Handle;
use hydrate_base::hashing::HashMap;
use hydrate_data::{ImporterId, RecordAccessor, RecordOwned};
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
        let source = std::fs::read_to_string(context.path)?;
        let json_format: HydrateMeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))?;

        let asset_type = context
            .schema_set
            .find_named_type(MeshAdvPrefabAssetAccessor::schema_name())?
            .as_record()?
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
        let source = std::fs::read_to_string(context.path)?;
        // We don't actually need to parse this now but worth doing to make sure it's well-formed at import time
        let _json_format: HydrateMeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))?;

        //
        // Create the default asset
        //
        let default_asset = MeshAdvPrefabAssetOwned::new_builder(context.schema_set);

        let import_data = MeshAdvPrefabImportDataOwned::new_builder(context.schema_set);
        import_data.json_data().set(source)?;

        //
        // Return the created objects
        //
        let mut imported_objects = HashMap::default();
        imported_objects.insert(
            None,
            ImportedImportable {
                file_references: Default::default(),
                import_data: Some(import_data.into_inner()?),
                default_asset: Some(default_asset.into_inner()?),
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
