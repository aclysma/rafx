use crate::slab::RawSlab;
use crate::render_nodes::*;
use crate::frame_packet::FramePacket;
use crate::features::sprite::*;
use crate::{RenderRegistry, RenderFeatureIndex};
use crate::RenderFeature;
use crate::features::static_quad::{StaticQuadRenderNode, StaticQuadRenderNodeHandle, StaticQuadRenderFeature};


pub trait RenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex;
    fn max_node_count(&self) -> usize;
}

pub struct AllRenderNodes<'a> {
    nodes: Vec<Option<&'a RenderNodeSet>>
}

impl<'a> AllRenderNodes<'a> {
    pub fn new() -> Self {
        let feature_count = RenderRegistry::registered_feature_count();
        let nodes = vec![None; feature_count as usize];

        AllRenderNodes {
            nodes
        }
    }

    pub fn add_render_nodes(&mut self, render_nodes: &'a RenderNodeSet) {
        self.nodes[render_nodes.feature_index() as usize] = Some(render_nodes);
    }

    pub fn max_node_count_by_type(&self) -> Vec<usize> {
        self.nodes.iter()
            .map(|node_set| node_set.map_or(0, |node_set| node_set.max_node_count())).collect()
    }
}


/*
////////////////// RenderNodeSet //////////////////
pub struct RenderNodeSet {
    sprites: RawSlab<SpriteRenderNode>,
    static_quads: RawSlab<StaticQuadRenderNode>,

    node_count_by_type: Vec<u32>,
}

impl RenderNodeSet {
    pub fn new() -> Self {
        RenderNodeSet {
            sprites: Default::default(),
            static_quads: Default::default(),
            node_count_by_type: vec![0, RenderRegistry::registered_feature_count()],
        }
    }

    pub fn node_count_by_type(&self) -> &[u32] {
        &self.node_count_by_type
    }

    pub fn register_sprite(
        &mut self,
        node: SpriteRenderNode,
    ) -> SpriteRenderNodeHandle {
        self.node_count_by_type[SpriteRenderFeature::feature_index() as usize] += 1;

        //TODO: Request streaming in a resource
        SpriteRenderNodeHandle(self.sprites.allocate(node))
    }

    pub fn unregister_sprite(
        &mut self,
        handle: SpriteRenderNodeHandle,
    ) {
        self.node_count_by_type[SpriteRenderFeature::feature_index() as usize] -= 1;

        //TODO: Decrement reference count for resource
        self.sprites.free(&handle.0);
    }

    pub fn register_static_quad(
        &mut self,
        node: StaticQuadRenderNode,
    ) -> StaticQuadRenderNodeHandle {
        self.node_count_by_type[StaticQuadRenderFeature::feature_index() as usize] += 1;

        //TODO: Request streaming in a resource
        StaticQuadRenderNodeHandle(self.static_quads.allocate(node))
    }

    pub fn unregister_static_quad(
        &mut self,
        handle: StaticQuadRenderNodeHandle,
    ) {
        self.node_count_by_type[StaticQuadRenderFeature::feature_index() as usize] -= 1;

        //TODO: Decrement reference count for resource
        self.static_quads.free(&handle.0);
    }

    // pub fn prepare(
    //     &self,
    //     frame_packet: &FramePacket,
    // ) {
    // }
    //
    // pub fn submit(
    //     &self,
    //     frame_packet: &FramePacket,
    // ) {
    // }
}
*/