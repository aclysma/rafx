use crate::assets::gltf::MeshAssetData;
use distill::loader::Loader;
use rafx::assets::AssetLookup;
use rafx::framework::MaterialPass;
use rafx::framework::{BufferResource, DescriptorSetArc, ResourceArc};
use std::sync::Arc;
use type_uuid::*;
use crate::assets::font::FontAsset;

pub struct MeshAssetPart {
    pub opaque_pass: MaterialPass,
    pub opaque_material_descriptor_set: DescriptorSetArc,
    // These are optional because we might want to disable casting shadows
    pub shadow_map_pass: Option<MaterialPass>,
    pub vertex_buffer_offset_in_bytes: u32,
    pub vertex_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
}

pub struct MeshAssetInner {
    pub mesh_parts: Vec<Option<MeshAssetPart>>,
    pub vertex_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
    pub asset_data: MeshAssetData,
}

#[derive(TypeUuid, Clone)]
#[uuid = "689a0bf0-e320-41c0-b4e8-bdb2055a7a57"]
pub struct MeshAsset {
    pub inner: Arc<MeshAssetInner>,
}

#[derive(Debug)]
pub struct GameLoadedAssetMetrics {
    pub mesh_count: usize,
    pub font_count: usize,
}

//
// Lookups by asset for loaded asset state
//
pub struct GameLoadedAssetLookupSet {
    pub meshes: AssetLookup<MeshAsset>,
    pub fonts: AssetLookup<FontAsset>,
}

impl GameLoadedAssetLookupSet {
    pub fn new(loader: &Loader) -> Self {
        GameLoadedAssetLookupSet {
            meshes: AssetLookup::new(loader),
            fonts: AssetLookup::new(loader),
        }
    }

    pub fn metrics(&self) -> GameLoadedAssetMetrics {
        GameLoadedAssetMetrics {
            mesh_count: self.meshes.len(),
            font_count: self.fonts.len(),
        }
    }

    pub fn destroy(&mut self) {
        self.meshes.destroy();
        self.fonts.destroy();
    }
}
