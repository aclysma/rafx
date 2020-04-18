use renderer_base::{RenderPhaseIndex, SubmitNode};
use std::sync::atomic::Ordering;
use renderer_base::RenderPhase;
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

    fn sort_submit_nodes(mut submit_nodes: Vec<SubmitNode>) -> Vec<SubmitNode> {
        // Sort by feature
        log::info!("Sort phase {}", Self::render_phase_debug_name());
        submit_nodes.sort_unstable_by(|a, b| a.feature_index().cmp(&b.feature_index()));

        submit_nodes
    }

    fn render_phase_debug_name() -> &'static str {
        "DrawOpaqueRenderPhase"
    }
}
