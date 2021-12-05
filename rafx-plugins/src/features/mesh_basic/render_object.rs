use super::MeshBasicRenderFeature;
use crate::assets::mesh::MeshAsset;
use distill::loader::handle::Handle;
use rafx::render_features::RenderObjectSet;

#[derive(Clone)]
pub struct MeshBasicRenderObject {
    pub mesh: Handle<MeshAsset>,
}

pub type MeshBasicRenderObjectSet = RenderObjectSet<MeshBasicRenderFeature, MeshBasicRenderObject>;
