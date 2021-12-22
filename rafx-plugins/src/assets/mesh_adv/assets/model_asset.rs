use distill::loader::handle::Handle;
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
#[uuid = "5fe1ac74-0f10-4855-aa4a-b95a3473020d"]
pub struct ModelBasicAssetData {
    pub lods: Vec<ModelBasicAssetDataLod>,
}

pub struct ModelBasicAssetInner {
    pub lods: Vec<ModelBasicAssetDataLod>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "82468fcb-7124-409d-bc74-140569aaebb4"]
pub struct ModelBasicAsset {
    pub inner: Arc<ModelBasicAssetInner>,
}

pub struct ModelBasicLoadHandler;

impl DefaultAssetTypeLoadHandler<ModelBasicAssetData, ModelBasicAsset> for ModelBasicLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        model_asset: ModelBasicAssetData,
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
