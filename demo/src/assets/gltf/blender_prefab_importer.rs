use crate::assets::gltf::{
    ModelAsset, PrefabAssetData, PrefabAssetDataObject, PrefabAssetDataObjectLight,
    PrefabAssetDataObjectLightKind, PrefabAssetDataObjectLightSpot, PrefabAssetDataObjectModel,
    PrefabAssetDataObjectTransform,
};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct PrefabJsonFormatObjectTransform {
    position: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrefabJsonFormatObjectModel {
    model: Handle<ModelAsset>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum PrefabJsonFormatObjectLightKind {
    Point,
    Spot,
    Directional,
}

impl Into<PrefabAssetDataObjectLightKind> for PrefabJsonFormatObjectLightKind {
    fn into(self) -> PrefabAssetDataObjectLightKind {
        match self {
            PrefabJsonFormatObjectLightKind::Point => PrefabAssetDataObjectLightKind::Point,
            PrefabJsonFormatObjectLightKind::Spot => PrefabAssetDataObjectLightKind::Spot,
            PrefabJsonFormatObjectLightKind::Directional => {
                PrefabAssetDataObjectLightKind::Directional
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrefabJsonFormatObjectLightSpot {
    inner_angle: f32,
    outer_angle: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrefabJsonFormatObjectLight {
    color: [f32; 3],
    kind: PrefabJsonFormatObjectLightKind,
    intensity: f32,
    spot: Option<PrefabJsonFormatObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrefabJsonFormatObject {
    transform: PrefabJsonFormatObjectTransform,
    model: Option<PrefabJsonFormatObjectModel>,
    light: Option<PrefabJsonFormatObjectLight>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PrefabJsonFormat {
    pub objects: Vec<PrefabJsonFormatObject>,
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "8fbf4a7e-5c86-4381-8e5d-61bc439fcf1a"]
pub struct BlenderPrefabImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ce0a6dc-51ee-4c67-be01-707c573cbdf1"]
pub struct BlenderPrefabImporter;
impl Importer for BlenderPrefabImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        3
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = BlenderPrefabImporterState;

    /// Reads the given bytes and produces assets.
    #[profiling::function]
    fn import(
        &self,
        _op: &mut ImportOp,
        source: &mut dyn Read,
        _options: &Self::Options,
        state: &mut Self::State,
    ) -> distill::importer::Result<ImporterValue> {
        let id = state
            .0
            .unwrap_or_else(|| AssetUuid(*uuid::Uuid::new_v4().as_bytes()));
        *state = BlenderPrefabImporterState(Some(id));

        let json_format: PrefabJsonFormat = serde_json::from_reader(source)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let mut objects = Vec::with_capacity(json_format.objects.len());
        for json_object in json_format.objects {
            let model = if let Some(json_model) = &json_object.model {
                let model = json_model.model.clone();

                Some(PrefabAssetDataObjectModel { model })
            } else {
                None
            };

            let light = if let Some(json_light) = &json_object.light {
                let light = json_light.clone();
                let spot = light.spot.as_ref().map(|x| PrefabAssetDataObjectLightSpot {
                    inner_angle: x.inner_angle,
                    outer_angle: x.outer_angle,
                });

                Some(PrefabAssetDataObjectLight {
                    color: light.color.into(),
                    kind: light.kind.into(),
                    intensity: light.intensity,
                    spot,
                })
            } else {
                None
            };

            let transform = PrefabAssetDataObjectTransform {
                position: json_object.transform.position.into(),
                rotation: json_object.transform.rotation.into(),
                scale: json_object.transform.scale.into(),
            };

            objects.push(PrefabAssetDataObject {
                transform,
                model,
                light,
            });
        }

        let asset_data = PrefabAssetData { objects };

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id,
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(asset_data),
            }],
        })
    }
}
