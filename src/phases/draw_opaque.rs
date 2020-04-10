use crate::registry::RenderPhaseIndex;
use std::sync::atomic::Ordering;
use crate::RenderPhase;
use std::sync::atomic::AtomicI32;
use std::convert::TryInto;

static DRAW_OPAQUE_RENDER_PHASE_INDEX: AtomicI32 = AtomicI32::new(-1);

pub struct DrawOpaqueRenderPhase;

impl RenderPhase for DrawOpaqueRenderPhase {
    fn set_render_phase_index(index: RenderPhaseIndex) {
        DRAW_OPAQUE_RENDER_PHASE_INDEX.store(index.try_into().unwrap(), Ordering::Release);
    }

    fn render_phase_index() -> RenderPhaseIndex {
        DRAW_OPAQUE_RENDER_PHASE_INDEX.load(Ordering::Acquire) as RenderPhaseIndex
    }
}
