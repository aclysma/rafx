use super::MeshAdvRenderFeature;
use crate::assets::mesh_adv::MeshAdvAsset;
use distill::loader::handle::Handle;
use rafx::render_features::RenderObjectSet;

#[derive(Clone)]
pub struct MeshAdvRenderObject {
    pub mesh: Handle<MeshAdvAsset>,
}

pub type MeshAdvRenderObjectSet = RenderObjectSet<MeshAdvRenderFeature, MeshAdvRenderObject>;
