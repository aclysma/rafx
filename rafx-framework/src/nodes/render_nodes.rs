use super::RenderFeatureIndex;
use super::RenderRegistry;
use rafx_base::slab::SlabIndexT;

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

pub struct RenderNodeReservations {
    max_render_nodes_by_feature: Vec<u32>,
}

impl Default for RenderNodeReservations {
    fn default() -> Self {
        let feature_count = RenderRegistry::registered_feature_count();
        let max_render_nodes_by_feature = vec![0; feature_count as usize];

        RenderNodeReservations {
            max_render_nodes_by_feature,
        }
    }
}

impl RenderNodeReservations {
    pub fn add_reservation(
        &mut self,
        render_nodes: &dyn RenderNodeSet,
    ) {
        // A panic here means a feature was not registered
        self.max_render_nodes_by_feature[render_nodes.feature_index() as usize] +=
            render_nodes.max_render_node_count();
    }

    pub fn max_render_nodes_by_feature(&self) -> &[u32] {
        &self.max_render_nodes_by_feature
    }
}
