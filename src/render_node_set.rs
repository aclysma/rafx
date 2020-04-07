
use crate::slab::RawSlab;
use crate::render_nodes::*;
use crate::frame_packet::FramePacket;
use crate::features::sprite::*;

////////////////// RenderNodeSet //////////////////
#[derive(Default)]
pub struct RenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>,
    static_quads: RawSlab<StaticQuadRenderNode>
}

impl RenderNodeSet {
    pub fn register_sprite(&mut self, node: SpriteRenderNode) -> SpriteRenderNodeHandle {
        //TODO: Request streaming in a resource
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn unregister_sprite(&mut self, handle: SpriteRenderNodeHandle) {
        //TODO: Decrement reference count for resource
        self.sprites.free(&handle.0);
    }

    pub fn register_static_quad(&mut self, node: StaticQuadRenderNode) -> StaticQuadRenderNodeHandle {
        //TODO: Request streaming in a resource
        StaticQuadRenderNodeHandle(self.static_quads.allocate(node))
    }

    pub fn unregister_static_quad(&mut self, handle: StaticQuadRenderNodeHandle) {
        //TODO: Decrement reference count for resource
        self.static_quads.free(&handle.0);
    }

    pub fn prepare(&self, frame_packet: &FramePacket) {

    }

    pub fn submit(&self, frame_packet: &FramePacket) {

    }
}