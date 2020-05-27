use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;

#[derive(TypeUuid, Serialize, Deserialize, Clone)]
#[uuid = "2d6653ce-5f77-40a2-b050-f2d148699d78"]
pub struct BufferAsset {
    pub data: Vec<u8>,
}

// enum VertexAttribute {
//     Position,
//     Normal,
//     TexCoord(u8)
// }
//
// #[derive(TypeUuid, Serialize, Deserialize)]
// #[uuid = "06d86f5e-caf9-4752-b710-17767406d965"]
// pub struct VertexSource {
//
// }
//
