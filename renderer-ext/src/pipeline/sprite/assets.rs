use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;

#[derive(Serialize, Deserialize, Debug)]
pub struct AnimationLayoutHorizontal {
    pub image_index: u32,
    pub frame_count: u32,
    pub images_count_per_row: u32,
    pub width: u32,
    pub height: u32,
    pub x_margin: u32,
    pub y_margin: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AnimationLayout {
    Horizontal(AnimationLayoutHorizontal)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Animation {
    pub name: String,
    pub frames: AnimationLayout
}

#[derive(TypeUuid, Serialize, Deserialize, Debug)]
#[uuid = "b06ff45c-7560-441a-a023-6a17707eeff4"]
pub struct SpriteAsset {
    pub images: Vec<AssetUuid>,
    pub animations: Vec<Animation>
}