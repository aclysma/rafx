use super::render_feature_index;
use distill::loader::handle::Handle;
use rafx::assets::ImageAsset;
use rafx::base::slab::{DropSlab, DropSlabKey};
use rafx::nodes::{GenericRenderNodeHandle, RenderFeatureIndex, RenderNodeCount, RenderNodeSet};

pub struct SpriteRenderNode {
    pub tint: glam::Vec3,
    pub alpha: f32,
    pub image: Handle<ImageAsset>,
}

#[derive(Clone)]
pub struct SpriteRenderNodeHandle(pub DropSlabKey<SpriteRenderNode>);

impl SpriteRenderNodeHandle {
    pub fn as_raw_generic_handle(&self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle::new(render_feature_index(), self.0.index())
    }
}

impl Into<GenericRenderNodeHandle> for SpriteRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        self.as_raw_generic_handle()
    }
}

#[derive(Default)]
pub struct SpriteRenderNodeSet {
    pub(super) sprites: DropSlab<SpriteRenderNode>,
}

impl SpriteRenderNodeSet {
    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn get_mut(
        &mut self,
        handle: &SpriteRenderNodeHandle,
    ) -> Option<&mut SpriteRenderNode> {
        self.sprites.get_mut(&handle.0)
    }

    pub fn update(&mut self) {
        self.sprites.process_drops();
    }
}

impl RenderNodeSet for SpriteRenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex {
        render_feature_index()
    }

    fn max_render_node_count(&self) -> RenderNodeCount {
        self.sprites.storage_size() as RenderNodeCount
    }
}
