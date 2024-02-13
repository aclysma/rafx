use crate::assets::mesh_adv::PrefabAdvAssetDataObjectLightKind;
use crate::schema::{MeshAdvPrefabAssetRecord, MeshAdvPrefabImportDataRecord};
use hydrate_data::Record;
use hydrate_pipeline::{
    AssetPlugin, AssetPluginSetupContext, ImportContext, Importer, PipelineResult, ScanContext,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectTransform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
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
pub struct MeshAdvPrefabJsonFormatObjectModel {
    pub model: PathBuf, //Handle<ModelAdvAsset>,
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
    ) -> PipelineResult<()> {
        //
        // Read the file
        //
        let source = std::fs::read_to_string(context.path)?;
        let json_format: MeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))?;

        let importable = context.add_default_importable::<MeshAdvPrefabAssetRecord>()?;

        for object in &json_format.objects {
            if let Some(model) = &object.model {
                importable.add_path_reference(&model.model)?;
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
        // We don't actually need to parse this now but worth doing to make sure it's well-formed at import time
        let _json_format: MeshAdvPrefabJsonFormat = serde_json::from_str(&source)
            .map_err(|x| format!("Blender Prefab Import error: {:?}", x))?;

        //
        // Create the default asset
        //
        let default_asset = MeshAdvPrefabAssetRecord::new_builder(context.schema_set);

        let import_data = MeshAdvPrefabImportDataRecord::new_builder(context.schema_set);
        import_data.json_data().set(source)?;

        //
        // Return the created objects
        //
        context
            .add_default_importable(default_asset.into_inner()?, Some(import_data.into_inner()?));
        Ok(())
    }
}

pub struct BlenderPrefabAssetPlugin;

impl AssetPlugin for BlenderPrefabAssetPlugin {
    fn setup(context: AssetPluginSetupContext) {
        context
            .importer_registry
            .register_handler::<BlenderPrefabImporter>();
    }
}
