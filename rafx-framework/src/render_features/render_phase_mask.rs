use crate::render_features::registry::{RenderPhaseMaskInnerType, MAX_RENDER_PHASE_COUNT};
use crate::render_features::{RenderPhase, RenderPhaseIndex};

#[derive(Default)]
pub struct RenderPhaseMaskBuilder(RenderPhaseMaskInnerType);

impl RenderPhaseMaskBuilder {
    pub fn add_render_phase<RenderPhaseT: RenderPhase>(mut self) -> RenderPhaseMaskBuilder {
        let index = RenderPhaseT::render_phase_index();
        assert!(
            index < MAX_RENDER_PHASE_COUNT,
            "render phase {} is not registered",
            RenderPhaseT::render_phase_debug_name()
        );
        self.0 |= 1 << RenderPhaseT::render_phase_index();
        self
    }

    pub fn build(self) -> RenderPhaseMask {
        RenderPhaseMask(self.0)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct RenderPhaseMask(RenderPhaseMaskInnerType);

impl RenderPhaseMask {
    pub fn is_included<RenderPhaseT: RenderPhase>(&self) -> bool {
        let index = RenderPhaseT::render_phase_index();
        assert!(
            index < MAX_RENDER_PHASE_COUNT,
            "render phase {} is not registered",
            RenderPhaseT::render_phase_debug_name()
        );

        self.is_included_index_unchecked(index)
    }

    #[inline(always)]
    pub fn is_included_index(
        &self,
        index: RenderPhaseIndex,
    ) -> bool {
        assert!(
            index < MAX_RENDER_PHASE_COUNT,
            "render phase index {} is invalid (did you forget to register a render phase?)",
            index
        );

        self.is_included_index_unchecked(index)
    }

    pub fn empty() -> Self {
        RenderPhaseMask(0)
    }

    #[inline(always)]
    fn is_included_index_unchecked(
        &self,
        index: RenderPhaseIndex,
    ) -> bool {
        (self.0 & 1 << index) != 0
    }
}
