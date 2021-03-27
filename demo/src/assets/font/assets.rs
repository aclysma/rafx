use fontdue::FontSettings;
use rafx::api::RafxResult;
use rafx::assets::{AssetManager, DefaultAssetTypeHandler, DefaultAssetTypeLoadHandler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use type_uuid::*;

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "197bfd7a-3df9-4440-86f0-8e10756c714e"]
pub struct FontAssetData {
    pub data_hash: u64,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
    pub scale: f32,
}

pub struct FontAssetInner {
    pub data_hash: u64,
    pub font: fontdue::Font,
    pub scale: f32,
}

#[derive(TypeUuid, Clone)]
#[uuid = "398689ef-4bf1-42ad-8fc4-5080c1b8293a"]
pub struct FontAsset {
    pub inner: Arc<FontAssetInner>,
}

pub struct FontLoadHandler;

impl DefaultAssetTypeLoadHandler<FontAssetData, FontAsset> for FontLoadHandler {
    #[profiling::function]
    fn load(
        _asset_manager: &mut AssetManager,
        font_asset: FontAssetData,
    ) -> RafxResult<FontAsset> {
        let settings = FontSettings::default();
        let font = fontdue::Font::from_bytes(font_asset.data.as_slice(), settings)
            .map_err(|x| x.to_string())?;

        let inner = FontAssetInner {
            font,
            data_hash: font_asset.data_hash,
            scale: font_asset.scale,
        };

        Ok(FontAsset {
            inner: Arc::new(inner),
        })
    }
}

pub type FontAssetType = DefaultAssetTypeHandler<FontAssetData, FontAsset, FontLoadHandler>;
