use glam::Mat4;
use crate::RenderPhase;
use crate::registry::{RenderPhaseMaskInnerType, MAX_RENDER_PHASE_COUNT};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Default)]
pub struct RenderPhaseMaskBuilder(RenderPhaseMaskInnerType);

impl RenderPhaseMaskBuilder {
    pub fn add_render_phase<T: RenderPhase>(mut self) -> RenderPhaseMaskBuilder {
        let index = T::render_phase_index();
        assert!(index < MAX_RENDER_PHASE_COUNT);
        self.0 |= 1 << T::render_phase_index();
        self
    }

    pub fn build(self) -> RenderPhaseMask {
        RenderPhaseMask(self.0)
    }
}

#[derive(Copy, Clone)]
pub struct RenderPhaseMask(RenderPhaseMaskInnerType);

#[derive(Default)]
pub struct RenderViewSet {
    view_count: AtomicUsize,
}

impl RenderViewSet {
    pub fn create_view(
        &self,
        view_proj: Mat4,
        render_stage_mask: RenderPhaseMask,
        debug_name: String,
    ) -> RenderView {
        let view_index = self.view_count.fetch_add(1, Ordering::Release);
        RenderView::new(view_index, view_proj, render_stage_mask, debug_name)
    }

    pub fn view_count(&self) -> usize {
        self.view_count.load(Ordering::Acquire)
    }
}

////////////////// Views //////////////////
pub struct RenderView {
    view_proj: Mat4,
    view_index: usize,
    render_stage_mask: RenderPhaseMask,
    debug_name: String, //visibility_results: Vec<Vec<GenericRenderNodeHandle>>,
}

impl RenderView {
    pub fn new(
        view_index: usize,
        view_proj: Mat4,
        render_stage_mask: RenderPhaseMask,
        debug_name: String,
    ) -> RenderView {
        log::debug!("Allocate view {} {}", debug_name, view_index);
        Self {
            view_proj,
            view_index,
            render_stage_mask,
            debug_name,
        }
    }

    pub fn view_index(&self) -> usize {
        self.view_index
    }

    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }
}
