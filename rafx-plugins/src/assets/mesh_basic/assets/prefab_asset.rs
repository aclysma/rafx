use distill::loader::handle::Handle;
use distill::loader::LoadHandle;
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
    pub range: Option<f32>,
    pub spot: Option<PrefabBasicAssetDataObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabBasicAssetDataObject {
    pub transform: PrefabBasicAssetDataObjectTransform,
    pub model: Option<PrefabBasicAssetDataObjectModel>,
    pub light: Option<PrefabBasicAssetDataObjectLight>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "1af63a91-de3e-48fc-8908-ab309730b8b5"]
pub struct PrefabBasicAssetData {
    pub objects: Vec<PrefabBasicAssetDataObject>,
}

pub struct PrefabBasicAssetInner {
    pub objects: Vec<PrefabBasicAssetDataObject>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "7bf45a97-62f4-4a9a-99b8-e0ac8d755993"]
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
        _load_handle: LoadHandle,
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
