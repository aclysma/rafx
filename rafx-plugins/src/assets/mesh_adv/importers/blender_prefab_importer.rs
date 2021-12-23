use crate::assets::mesh_adv::{
    ModelAdvAsset, PrefabAdvAssetData, PrefabAdvAssetDataObject, PrefabAdvAssetDataObjectLight,
    PrefabAdvAssetDataObjectLightKind, PrefabAdvAssetDataObjectLightSpot,
    PrefabAdvAssetDataObjectModel, PrefabAdvAssetDataObjectTransform,
};
use distill::importer::{ImportedAsset, Importer, ImporterValue};
use distill::{core::AssetUuid, importer::ImportOp};
use rafx::distill::loader::handle::Handle;
use serde::{Deserialize, Serialize};
use std::io::Read;
use type_uuid::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectTransform {
    position: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectModel {
    model: Handle<ModelAdvAsset>,
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
    inner_angle: f32,
    outer_angle: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObjectLight {
    color: [f32; 3],
    kind: MeshAdvPrefabJsonFormatObjectLightKind,
    intensity: f32,
    cutoff_distance: Option<f32>,
    spot: Option<MeshAdvPrefabJsonFormatObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshAdvPrefabJsonFormatObject {
    transform: MeshAdvPrefabJsonFormatObjectTransform,
    model: Option<MeshAdvPrefabJsonFormatObjectModel>,
    light: Option<MeshAdvPrefabJsonFormatObjectLight>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MeshAdvPrefabJsonFormat {
    pub objects: Vec<MeshAdvPrefabJsonFormatObject>,
}

#[derive(TypeUuid, Serialize, Deserialize, Default)]
#[uuid = "5f9022bc-fd83-4f99-9fb7-a395fd997361"]
pub struct MeshAdvBlenderPrefabImporterState(Option<AssetUuid>);

#[derive(TypeUuid)]
#[uuid = "1441f5a2-5c3b-404b-b03f-2234146e2c2f"]
pub struct MeshAdvBlenderPrefabImporter;
impl Importer for MeshAdvBlenderPrefabImporter {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        4
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();

    type State = MeshAdvBlenderPrefabImporterState;

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
        *state = MeshAdvBlenderPrefabImporterState(Some(id));

        let json_format: MeshAdvPrefabJsonFormat = serde_json::from_reader(source)
            .map_err(|x| format!("Blender Material Import error: {:?}", x))?;

        let mut objects = Vec::with_capacity(json_format.objects.len());
        for json_object in json_format.objects {
            let model = if let Some(json_model) = &json_object.model {
                let model = json_model.model.clone();

                Some(PrefabAdvAssetDataObjectModel { model })
            } else {
                None
            };

            let light = if let Some(json_light) = &json_object.light {
                let light = json_light.clone();
                let spot = light
                    .spot
                    .as_ref()
                    .map(|x| PrefabAdvAssetDataObjectLightSpot {
                        inner_angle: x.inner_angle,
                        outer_angle: x.outer_angle,
                    });

                let range = if light.cutoff_distance.unwrap_or(-1.0) < 0.0 {
                    None
                } else {
                    light.cutoff_distance
                };
                Some(PrefabAdvAssetDataObjectLight {
                    color: light.color.into(),
                    kind: light.kind.into(),
                    intensity: light.intensity,
                    range,
                    spot,
                })
            } else {
                None
            };

            let transform = PrefabAdvAssetDataObjectTransform {
                position: json_object.transform.position.into(),
                rotation: json_object.transform.rotation.into(),
                scale: json_object.transform.scale.into(),
            };

            objects.push(PrefabAdvAssetDataObject {
                transform,
                model,
                light,
            });
        }

        let asset_data = PrefabAdvAssetData { objects };

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
