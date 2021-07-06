use distill::loader::handle::Handle;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

use super::MeshAsset;

#[derive(Serialize, Deserialize, Clone)]
pub struct ModelAssetDataLod {
    pub mesh: Handle<MeshAsset>,
}

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "75bbc873-e527-42c6-8409-15aa5e68a4a4"]
pub struct ModelAssetData {
    pub lods: Vec<ModelAssetDataLod>,
}

pub struct ModelAssetInner {
    pub lods: Vec<ModelAssetDataLod>,
}

#[derive(TypeUuid, Clone)]
#[uuid = "76b953ef-9d1e-464b-b2a8-74f5b8842bd8"]
pub struct ModelAsset {
    pub inner: Arc<ModelAssetInner>,
}

pub struct ModelLoadHandler;

impl DefaultAssetTypeLoadHandler<ModelAssetData, ModelAsset> for ModelLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        model_asset: ModelAssetData,
    ) -> RafxResult<ModelAsset> {
        let inner = ModelAssetInner {
            lods: model_asset.lods,
        };

        Ok(ModelAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type ModelAssetType = DefaultAssetTypeHandler<ModelAssetData, ModelAsset, ModelLoadHandler>;
