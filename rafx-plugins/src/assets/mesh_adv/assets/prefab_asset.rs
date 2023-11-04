use distill::loader::handle::Handle;
use distill::loader::LoadHandle;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::ModelAdvAsset;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAdvAssetDataObjectTransform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAdvAssetDataObjectModel {
    pub model: Handle<ModelAdvAsset>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PrefabAdvAssetDataObjectLightKind {
    Point,
    Spot,
    Directional,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAdvAssetDataObjectLightSpot {
    pub inner_angle: f32,
    pub outer_angle: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAdvAssetDataObjectLight {
    pub color: glam::Vec3,
    pub kind: PrefabAdvAssetDataObjectLightKind,
    pub intensity: f32,
    pub range: Option<f32>,
    pub spot: Option<PrefabAdvAssetDataObjectLightSpot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrefabAdvAssetDataObject {
    pub transform: PrefabAdvAssetDataObjectTransform,
    pub model: Option<PrefabAdvAssetDataObjectModel>,
    pub light: Option<PrefabAdvAssetDataObjectLight>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "2aa26beb-2359-4f57-a035-8e33b3ce8bf1"]
pub struct PrefabAdvAssetData {
    pub objects: Vec<PrefabAdvAssetDataObject>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HydratePrefabAdvAssetDataObjectModel {
    pub model: hydrate_base::Handle<ModelAdvAsset>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HydratePrefabAdvAssetDataObject {
    pub transform: PrefabAdvAssetDataObjectTransform,
    pub model: Option<HydratePrefabAdvAssetDataObjectModel>,
    pub light: Option<PrefabAdvAssetDataObjectLight>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone, Debug)]
#[uuid = "fcbd5421-7ea0-4270-9d67-f06cbd0c08e1"]
pub struct HydratePrefabAdvAssetData {
    pub objects: Vec<HydratePrefabAdvAssetDataObject>,
}

pub struct PrefabAdvAssetInner {
    pub objects: Vec<PrefabAdvAssetDataObject>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "ad1525bc-802b-4574-bac3-2a387f328d14"]
pub struct PrefabAdvAsset {
    pub inner: Arc<PrefabAdvAssetInner>,
}

pub struct PrefabAdvLoadHandler;

impl DefaultAssetTypeLoadHandler<PrefabAdvAssetData, PrefabAdvAsset> for PrefabAdvLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        model_asset: PrefabAdvAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<PrefabAdvAsset> {
        let inner = PrefabAdvAssetInner {
            objects: model_asset.objects,
        };

        Ok(PrefabAdvAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type PrefabAdvAssetType =
    DefaultAssetTypeHandler<PrefabAdvAssetData, PrefabAdvAsset, PrefabAdvLoadHandler>;
