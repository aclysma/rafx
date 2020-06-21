use crate::features::mesh::MeshRenderNodeHandle;
use renderer_visibility::DynamicAabbVisibilityNodeHandle;
use atelier_assets::loader::handle::Handle;
use crate::assets::gltf::MeshAsset;

#[derive(Clone)]
pub struct MeshComponent {
    pub mesh_handle: MeshRenderNodeHandle,
    pub visibility_handle: DynamicAabbVisibilityNodeHandle,
    pub mesh: Handle<MeshAsset>,
}
