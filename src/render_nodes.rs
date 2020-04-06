
use renderer_base::slab::RawSlabKey;

////////////////// Sprite RenderNode //////////////////
pub struct SpriteRenderNode {
    // entity handle
    // texture
}

#[derive(Copy, Clone)]
pub struct SpriteRenderNodeHandle(pub RawSlabKey<SpriteRenderNode>);

impl Into<GenericRenderNodeHandle> for SpriteRenderNodeHandle {
    fn into(self) -> GenericRenderNodeHandle {
        GenericRenderNodeHandle {
            node_type: RenderNodeType::Sprite,
            handle: self.0.index()
        }
    }
}

////////////////// StaticQuad RenderNode //////////////////
pub struct StaticQuadRenderNode {
    // texture
    // location
}

pub struct StaticQuadRenderNodeHandle(pub RawSlabKey<StaticQuadRenderNode>);


//This probably wouldn't be an enum since we'd like to let people add their own types non-intrusively
#[derive(Copy, Clone)]
pub enum RenderNodeType {
    Sprite,
    StaticQuad
}

#[derive(Copy, Clone)]
pub struct GenericRenderNodeHandle {
    node_type: RenderNodeType,
    handle: renderer_base::slab::SlabIndexT
}
