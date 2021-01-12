use crate::metal::RenderpassDef;

pub struct RafxRenderpassMetal {
    pub(crate) def: RenderpassDef,
}

impl RafxRenderpassMetal {
    pub fn new(def: RenderpassDef) -> Self {
        RafxRenderpassMetal { def }
    }
}
