use crate::assets::mesh_basic::{
    ModelBasicAsset, PrefabBasicAssetData, PrefabBasicAssetDataObject,
    PrefabBasicAssetDataObjectLight, PrefabBasicAssetDataObjectLightKind,
    PrefabBasicAssetDataObjectLightSpot, PrefabBasicAssetDataObjectModel,
    PrefabBasicAssetDataObjectTransform,
};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshBasicPrefabJsonFormatObjectTransform {
    position: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshBasicPrefabJsonFormatObjectModel {
    model: Handle<ModelBasicAsset>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum MeshBasicPrefabJsonFormatObjectLightKind {
    Point,
    Spot,
    Directional,
}

impl Into<PrefabBasicAssetDataObjectLightKind> for MeshBasicPrefabJsonFormatObjectLightKind {
    fn into(self) -> PrefabBasicAssetDataObjectLightKind {
        match self {
            MeshBasicPrefabJsonFormatObjectLightKind::Point => {
                PrefabBasicAssetDataObjectLightKind::Point
            }
            MeshBasicPrefabJsonFormatObjectLightKind::Spot => {
                PrefabBasicAssetDataObjectLightKind::Spot
            }
            MeshBasicPrefabJsonFormatObjectLightKind::Directional => {
                PrefabBasicAssetDataObjectLightKind::Directional
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshBasicPrefabJsonFormatObjectLightSpot {
    inner_angle: f32,
    outer_angle: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshBasicPrefabJsonFormatObjectLight {
    color: [f32; 3],
    kind: MeshBasicPrefabJsonFormatObjectLightKind,
    intensity: f32,
    cutoff_distance: f32,
    spot: Option<MeshBasicPrefabJsonFormatObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshBasicPrefabJsonFormatObject {
    transform: MeshBasicPrefabJsonFormatObjectTransform,
    model: Option<MeshBasicPrefabJsonFormatObjectModel>,
    light: Option<MeshBasicPrefabJsonFormatObjectLight>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshBasicPrefabJsonFormat {
    pub objects: Vec<MeshBasicPrefabJsonFormatObject>,
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "8fbf4a7e-5c86-4381-8e5d-61bc439fcf1a"]
pub struct MeshBasicBlenderPrefabImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "4ce0a6dc-51ee-4c67-be01-707c573cbdf1"]
pub struct MeshBasicBlenderPrefabImporter;
impl Importer for MeshBasicBlenderPrefabImporter {
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

    type State = MeshBasicBlenderPrefabImporterState;

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
        *state = MeshBasicBlenderPrefabImporterState(Some(id));

        let json_format: MeshBasicPrefabJsonFormat = serde_json::from_reader(source)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let mut objects = Vec::with_capacity(json_format.objects.len());
        for json_object in json_format.objects {
            let model = if let Some(json_model) = &json_object.model {
                let model = json_model.model.clone();

                Some(PrefabBasicAssetDataObjectModel { model })
            } else {
                None
            };

            let light = if let Some(json_light) = &json_object.light {
                let light = json_light.clone();
                let spot = light
                    .spot
                    .as_ref()
                    .map(|x| PrefabBasicAssetDataObjectLightSpot {
                        inner_angle: x.inner_angle,
                        outer_angle: x.outer_angle,
                    });

                let range = if light.cutoff_distance < 0.0 {
                    None
                } else {
                    Some(light.cutoff_distance)
                };
                Some(PrefabBasicAssetDataObjectLight {
                    color: light.color.into(),
                    kind: light.kind.into(),
                    intensity: light.intensity,
                    range,
                    spot,
                })
            } else {
                None
            };

            let transform = PrefabBasicAssetDataObjectTransform {
                position: json_object.transform.position.into(),
                rotation: json_object.transform.rotation.into(),
                scale: json_object.transform.scale.into(),
            };

            objects.push(PrefabBasicAssetDataObject {
                transform,
                model,
                light,
            });
        }

        let asset_data = PrefabBasicAssetData { objects };

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
