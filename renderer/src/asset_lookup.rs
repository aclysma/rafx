use renderer_resources::resource_managers::{DescriptorSetArc, ResourceArc, AssetLookup};
use renderer_shell_vulkan::VkBufferRaw;
use crate::assets::gltf::MeshAsset;

pub struct LoadedMeshPart {
    //pub material: ResourceArc<LoadedMaterial>,
    pub material_instance: Vec<Vec<DescriptorSetArc>>,
}

pub struct LoadedMesh {
    pub mesh_parts: Vec<LoadedMeshPart>,
    pub vertex_buffer: ResourceArc<VkBufferRaw>,
    pub index_buffer: ResourceArc<VkBufferRaw>,
    pub asset: MeshAsset,
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
    pub meshes: AssetLookup<LoadedMesh>,
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
