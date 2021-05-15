use super::MeshRenderFeature;
use crate::assets::gltf::MeshAsset;
use distill::loader::handle::Handle;
use rafx::render_features::RenderObjectSet;

#[derive(Clone)]
pub struct MeshRenderObject {
    pub mesh: Handle<MeshAsset>,
}

pub type MeshRenderObjectSet = RenderObjectSet<MeshRenderFeature, MeshRenderObject>;
