use renderer::assets::resource_managers::{DescriptorSetArc, ResourceArc, AssetLookup};
use renderer::vulkan::VkBufferRaw;
use crate::assets::gltf::MeshAssetData;
use type_uuid::*;
use std::sync::Arc;

pub struct MeshAssetPart {
    //pub material: ResourceArc<LoadedMaterial>,
    pub material_instance: Arc<Vec<Vec<DescriptorSetArc>>>,
}

pub struct MeshAssetInner {
    pub mesh_parts: Vec<MeshAssetPart>,
    pub vertex_buffer: ResourceArc<VkBufferRaw>,
    pub index_buffer: ResourceArc<VkBufferRaw>,
    pub asset: MeshAssetData,
}

#[derive(TypeUuid, Clone)]
#[uuid = "689a0bf0-e320-41c0-b4e8-bdb2055a7a57"]
pub struct MeshAsset {
    pub inner: Arc<MeshAssetInner>
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
