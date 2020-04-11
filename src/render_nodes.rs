use crate::RenderFeatureIndex;
use crate::slab::SlabIndexT;
use std::fmt::{Debug, Formatter, Error};

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
