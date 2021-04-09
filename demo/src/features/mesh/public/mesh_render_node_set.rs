use super::super::render_feature_index;
use crate::assets::gltf::MeshAsset;
use distill::loader::handle::Handle;
use rafx::base::slab::{DropSlab, DropSlabKey};
use rafx::nodes::{GenericRenderNodeHandle, RenderFeatureIndex, RenderNodeCount, RenderNodeSet};

//
// This is boiler-platish
//
pub struct MeshRenderNode {
    pub mesh: Option<Handle<MeshAsset>>,
    pub transform: glam::Mat4,
}

#[derive(Clone)]
pub struct MeshRenderNodeHandle(pub DropSlabKey<MeshRenderNode>);

impl MeshRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(render_feature_index(), self.0.index())
    }
}

impl Into<GenericRenderNodeHandle> for MeshRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct MeshRenderNodeSet {
    pub(in crate::features::mesh) meshes: DropSlab<MeshRenderNode>,
}

impl MeshRenderNodeSet {
    pub fn register_mesh(
        &mut self,
        node: MeshRenderNode,
    ) -> MeshRenderNodeHandle {
        MeshRenderNodeHandle(self.meshes.allocate(node))
    }

    pub fn get_mut(
        &mut self,
        handle: &MeshRenderNodeHandle,
    ) -> Option<&mut MeshRenderNode> {
        self.meshes.get_mut(&handle.0)
    }

    pub fn update(&mut self) {
        self.meshes.process_drops();
    }
}

impl RenderNodeSet for MeshRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.meshes.storage_size() as RenderNodeCount
    }
}
