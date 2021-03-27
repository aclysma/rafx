use super::RenderFeatureIndex;
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
