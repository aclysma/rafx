use super::MeshBasicRenderFeature;
use crate::assets::mesh_basic::MeshBasicAsset;
use hydrate_base::handle::Handle;
use rafx::render_features::RenderObjectSet;

#[derive(Clone)]
pub struct MeshBasicRenderObject {
    pub mesh: Handle<MeshBasicAsset>,
}

pub type MeshBasicRenderObjectSet = RenderObjectSet<MeshBasicRenderFeature, MeshBasicRenderObject>;
