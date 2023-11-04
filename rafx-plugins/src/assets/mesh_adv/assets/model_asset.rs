use distill::loader::handle::Handle;
use distill::loader::LoadHandle;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::MeshAdvAsset;

#[derive(Serialize, Deserialize, Clone)]
pub struct ModelAdvAssetDataLod {
    pub mesh: Handle<MeshAdvAsset>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "5fe1ac74-0f10-4855-aa4a-b95a3473020d"]
pub struct ModelAdvAssetData {
    pub lods: Vec<ModelAdvAssetDataLod>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HydrateModelAdvAssetDataLod {
    pub mesh: hydrate_base::Handle<MeshAdvAsset>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "5c6d78ae-ad2b-4cdc-a7a7-8ade67882c6e"]
pub struct HydrateModelAdvAssetData {
    pub lods: Vec<HydrateModelAdvAssetDataLod>,
}

pub struct ModelAdvAssetInner {
    pub lods: Vec<ModelAdvAssetDataLod>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "82468fcb-7124-409d-bc74-140569aaebb4"]
pub struct ModelAdvAsset {
    pub inner: Arc<ModelAdvAssetInner>,
}

pub struct ModelAdvLoadHandler;

impl DefaultAssetTypeLoadHandler<ModelAdvAssetData, ModelAdvAsset> for ModelAdvLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        model_asset: ModelAdvAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<ModelAdvAsset> {
        let inner = ModelAdvAssetInner {
            lods: model_asset.lods,
        };

        Ok(ModelAdvAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type ModelAdvAssetType =
    DefaultAssetTypeHandler<ModelAdvAssetData, ModelAdvAsset, ModelAdvLoadHandler>;
