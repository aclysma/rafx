use distill::loader::handle::Handle;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::ModelBasicAsset;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabBasicAssetDataObjectTransform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabBasicAssetDataObjectModel {
    pub model: Handle<ModelBasicAsset>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PrefabBasicAssetDataObjectLightKind {
    Point,
    Spot,
    Directional,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabBasicAssetDataObjectLightSpot {
    pub inner_angle: f32,
    pub outer_angle: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabBasicAssetDataObjectLight {
    pub color: glam::Vec3,
    pub kind: PrefabBasicAssetDataObjectLightKind,
    pub intensity: f32,
    pub spot: Option<PrefabBasicAssetDataObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabBasicAssetDataObject {
    pub transform: PrefabBasicAssetDataObjectTransform,
    pub model: Option<PrefabBasicAssetDataObjectModel>,
    pub light: Option<PrefabBasicAssetDataObjectLight>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "2aa26beb-2359-4f57-a035-8e33b3ce8bf1"]
pub struct PrefabBasicAssetData {
    pub objects: Vec<PrefabBasicAssetDataObject>,
}

pub struct PrefabBasicAssetInner {
    pub objects: Vec<PrefabBasicAssetDataObject>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "ad1525bc-802b-4574-bac3-2a387f328d14"]
pub struct PrefabBasicAsset {
    pub inner: Arc<PrefabBasicAssetInner>,
}

pub struct PrefabBasicLoadHandler;

impl DefaultAssetTypeLoadHandler<PrefabBasicAssetData, PrefabBasicAsset>
    for PrefabBasicLoadHandler
{
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        model_asset: PrefabBasicAssetData,
    ) -> RafxResult<PrefabBasicAsset> {
        let inner = PrefabBasicAssetInner {
            objects: model_asset.objects,
        };

        Ok(PrefabBasicAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type PrefabBasicAssetType =
    DefaultAssetTypeHandler<PrefabBasicAssetData, PrefabBasicAsset, PrefabBasicLoadHandler>;
