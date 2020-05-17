use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;

#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "e6166902-8716-401b-9d2e-8b01701c5626"]
pub struct ImageAsset {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}
