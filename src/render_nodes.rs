use crate::RenderFeatureIndex;
use crate::slab::SlabIndexT;
use crate::slab::RawSlab;
use crate::frame_packet::FramePacket;
use crate::features::sprite::*;
use crate::RenderRegistry;
use crate::RenderFeature;
use crate::features::static_quad::{StaticQuadRenderNode, StaticQuadRenderNodeHandle, StaticQuadRenderFeature};

#[derive(Copy, Clone, Debug)]
pub struct GenericRenderNodeHandle {
    render_feature_index: RenderFeatureIndex,
    slab_index: SlabIndexT,
}

impl GenericRenderNodeHandle {
    pub fn new(
        render_feature_index: RenderFeatureIndex,
        slab_index: SlabIndexT,
    ) -> Self {
        GenericRenderNodeHandle {
            render_feature_index,
            slab_index,
        }
    }

    pub fn render_feature_index(&self) -> RenderFeatureIndex {
        self.render_feature_index
    }

    pub fn slab_index(&self) -> SlabIndexT {
        self.slab_index
    }
}

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
