use atelier_assets::core::AssetUuid;
use atelier_assets::importer::{
    Error, ImportedAsset, Importer, ImporterValue, Result, SourceFileImporter,
};
use image2::{color, ImageBuf, Image};
use serde::{Deserialize, Serialize};
use type_uuid::*;
use std::io::Read;
use std::convert::TryInto;
use crate::pipeline::buffer::BufferAsset;
use atelier_assets::loader::handle::Handle;
use crate::pipeline::image::ImageAsset;

#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "130a91a8-ba80-4cad-9bce-848326b234c7"]
pub struct GltfMaterialAsset {
    pub base_color: [f32; 4],
    pub base_color_texture: Option<Handle<ImageAsset>>,
}

/// Vertex format for vertices sent to the GPU
#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
#[repr(packed(1))]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

#[derive(Serialize, Deserialize)]
pub struct MeshPart {
    pub vertex_buffer_offset_in_bytes: u32,
    pub vertex_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
    pub material: Option<Handle<GltfMaterialAsset>>,
}

#[derive(TypeUuid, Serialize, Deserialize)]
#[uuid = "cf232526-3757-4d94-98d1-c2f7e27c979f"]
pub struct MeshAsset {
    pub mesh_parts: Vec<MeshPart>,
    pub vertex_buffer: Handle<BufferAsset>, //Vec<MeshVertex>,
    pub index_buffer: Handle<BufferAsset>, //Vec<u16>,
}
