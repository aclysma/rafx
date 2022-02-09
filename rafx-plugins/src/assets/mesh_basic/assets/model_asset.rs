use distill::loader::handle::Handle;
use distill::loader::LoadHandle;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::MeshBasicAsset;

#[derive(Serialize, Deserialize, Clone)]
pub struct ModelBasicAssetDataLod {
    pub mesh: Handle<MeshBasicAsset>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "75bbc873-e527-42c6-8409-15aa5e68a4a4"]
pub struct ModelBasicAssetData {
    pub lods: Vec<ModelBasicAssetDataLod>,
}

pub struct ModelBasicAssetInner {
    pub lods: Vec<ModelBasicAssetDataLod>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "76b953ef-9d1e-464b-b2a8-74f5b8842bd8"]
pub struct ModelBasicAsset {
    pub inner: Arc<ModelBasicAssetInner>,
}

pub struct ModelBasicLoadHandler;

impl DefaultAssetTypeLoadHandler<ModelBasicAssetData, ModelBasicAsset> for ModelBasicLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        model_asset: ModelBasicAssetData,
        _load_handle: LoadHandle,
    ) -> RafxResult<ModelBasicAsset> {
        let inner = ModelBasicAssetInner {
            lods: model_asset.lods,
        };

        Ok(ModelBasicAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type ModelBasicAssetType =
    DefaultAssetTypeHandler<ModelBasicAssetData, ModelBasicAsset, ModelBasicLoadHandler>;
