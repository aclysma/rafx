use renderer::assets::resources::{DescriptorSetArc, ResourceArc, BufferResource};
use renderer::assets::AssetLookup;
use crate::assets::gltf::MeshAssetData;
use renderer::assets::MaterialPass;
use type_uuid::*;
use std::sync::Arc;
use atelier_assets::loader::Loader;

pub struct MeshAssetPart {
    pub opaque_pass: MaterialPass,
    pub opaque_material_descriptor_set: DescriptorSetArc,
    // These are optional because we might want to disable casting shadows
    pub shadow_map_pass: Option<MaterialPass>,
    pub shadow_map_material_descriptor_set: Option<DescriptorSetArc>,
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
}

//
// Lookups by asset for loaded asset state
//
pub struct GameLoadedAssetLookupSet {
    pub meshes: AssetLookup<MeshAsset>,
}

impl GameLoadedAssetLookupSet {
    pub fn new(loader: &Loader) -> Self {
        GameLoadedAssetLookupSet {
            meshes: AssetLookup::new(loader),
        }
    }

    pub fn metrics(&self) -> GameLoadedAssetMetrics {
        GameLoadedAssetMetrics {
            mesh_count: self.meshes.len(),
        }
    }

    pub fn destroy(&mut self) {
        self.meshes.destroy();
    }
}
