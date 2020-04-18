use crate::RenderFeatureIndex;
use crate::slab::SlabIndexT;
use crate::RenderRegistry;

pub type RenderNodeIndex = u32;
pub type RenderNodeCount = u32;

#[derive(Copy, Clone, Debug)]
pub struct GenericRenderNodeHandle {
    render_feature_index: RenderFeatureIndex,
    render_node_index: SlabIndexT,
}

impl GenericRenderNodeHandle {
    pub fn new(
        render_feature_index: RenderFeatureIndex,
        render_node_index: SlabIndexT,
    ) -> Self {
        GenericRenderNodeHandle {
            render_feature_index,
            render_node_index,
        }
    }

    pub fn render_feature_index(self) -> RenderFeatureIndex {
        self.render_feature_index
    }

    pub fn render_node_index(self) -> SlabIndexT {
        self.render_node_index
    }
}

pub trait RenderNodeSet {
    fn feature_index(&self) -> RenderFeatureIndex;
    fn max_render_node_count(&self) -> RenderNodeCount;
}

pub struct AllRenderNodes<'a> {
    nodes: Vec<Option<&'a dyn RenderNodeSet>>,
}

impl<'a> AllRenderNodes<'a> {
    pub fn new() -> Self {
        let feature_count = RenderRegistry::registered_feature_count();
        let nodes = vec![None; feature_count as usize];

        AllRenderNodes { nodes }
    }

    pub fn add_render_nodes(
        &mut self,
        render_nodes: &'a dyn RenderNodeSet,
    ) {
        self.nodes[render_nodes.feature_index() as usize] = Some(render_nodes);
    }

    pub fn max_render_node_count_by_type(&self) -> Vec<RenderNodeCount> {
        self.nodes
            .iter()
            .map(|node_set| node_set.map_or(0, |node_set| node_set.max_render_node_count()))
            .collect()
    }
}
