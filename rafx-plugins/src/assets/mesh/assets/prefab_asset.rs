use distill::loader::handle::Handle;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::ModelAsset;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAssetDataObjectTransform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAssetDataObjectModel {
    pub model: Handle<ModelAsset>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PrefabAssetDataObjectLightKind {
    Point,
    Spot,
    Directional,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAssetDataObjectLightSpot {
    pub inner_angle: f32,
    pub outer_angle: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAssetDataObjectLight {
    pub color: glam::Vec3,
    pub kind: PrefabAssetDataObjectLightKind,
    pub intensity: f32,
    pub spot: Option<PrefabAssetDataObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAssetDataObject {
    pub transform: PrefabAssetDataObjectTransform,
    pub model: Option<PrefabAssetDataObjectModel>,
    pub light: Option<PrefabAssetDataObjectLight>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "1af63a91-de3e-48fc-8908-ab309730b8b5"]
pub struct PrefabAssetData {
    pub objects: Vec<PrefabAssetDataObject>,
}

pub struct PrefabAssetInner {
    pub objects: Vec<PrefabAssetDataObject>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "7bf45a97-62f4-4a9a-99b8-e0ac8d755993"]
pub struct PrefabAsset {
    pub inner: Arc<PrefabAssetInner>,
}

pub struct PrefabLoadHandler;

impl DefaultAssetTypeLoadHandler<PrefabAssetData, PrefabAsset> for PrefabLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        model_asset: PrefabAssetData,
    ) -> RafxResult<PrefabAsset> {
        let inner = PrefabAssetInner {
            objects: model_asset.objects,
        };

        Ok(PrefabAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type PrefabAssetType = DefaultAssetTypeHandler<PrefabAssetData, PrefabAsset, PrefabLoadHandler>;
