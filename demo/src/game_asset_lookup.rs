use renderer::assets::resources::{DescriptorSetArc, ResourceArc, AssetLookup, BufferResource};
use crate::assets::gltf::MeshAssetData;
use renderer::assets::assets::MaterialPass;
use type_uuid::*;
use std::sync::Arc;

pub struct MeshAssetPart {
    pub material_passes: Arc<Vec<MaterialPass>>,
    pub material_instance_descriptor_sets: Arc<Vec<Vec<DescriptorSetArc>>>,
    pub vertex_buffer_offset_in_bytes: u32,
    pub vertex_buffer_size_in_bytes: u32,
    pub index_buffer_offset_in_bytes: u32,
    pub index_buffer_size_in_bytes: u32,
}

pub struct MeshAssetInner {
    pub mesh_parts: Vec<MeshAssetPart>,
    pub vertex_buffer: ResourceArc<BufferResource>,
    pub index_buffer: ResourceArc<BufferResource>,
    pub asset: MeshAssetData,
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
#[derive(Default)]
pub struct GameLoadedAssetLookupSet {
    pub meshes: AssetLookup<MeshAsset>,
}

impl GameLoadedAssetLookupSet {
    pub fn metrics(&self) -> GameLoadedAssetMetrics {
        GameLoadedAssetMetrics {
            mesh_count: self.meshes.len(),
        }
    }

    pub fn destroy(&mut self) {
        self.meshes.destroy();
    }
}
