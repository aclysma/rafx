use glam::{Mat4, Vec3};
use crate::RenderPhase;
use crate::registry::{RenderPhaseMaskInnerType, MAX_RENDER_PHASE_COUNT};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

pub type RenderViewIndex = u32;
pub type RenderViewCount = u32;

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

#[derive(Copy, Clone, Debug)]
pub struct RenderPhaseMask(RenderPhaseMaskInnerType);

impl RenderPhaseMask {
    pub fn is_included<RenderPhaseT: RenderPhase>(&self) -> bool {
        (self.0 & 1 << RenderPhaseT::render_phase_index()) != 0
    }
}

#[derive(Default)]
pub struct RenderViewSet {
    view_count: AtomicU32,
}

impl RenderViewSet {
    pub fn create_view(
        &self,
        eye_position: Vec3,
        view_proj: Mat4,
        render_phase_mask: RenderPhaseMask,
        debug_name: String,
    ) -> RenderView {
        let view_index = self.view_count.fetch_add(1, Ordering::Release);
        RenderView::new(
            view_index,
            eye_position,
            view_proj,
            render_phase_mask,
            debug_name,
        )
    }

    pub fn view_count(&self) -> RenderViewCount {
        self.view_count.load(Ordering::Acquire)
    }
}

////////////////// Views //////////////////
pub struct RenderView {
    eye_position: Vec3,
    view_proj: Mat4,
    view_index: RenderViewIndex,
    render_phase_mask: RenderPhaseMask,
    debug_name: String,
}

impl RenderView {
    pub fn new(
        view_index: RenderViewIndex,
        eye_position: Vec3,
        view_proj: Mat4,
        render_phase_mask: RenderPhaseMask,
        debug_name: String,
    ) -> RenderView {
        log::debug!("Allocate view {} {}", debug_name, view_index);
        Self {
            eye_position,
            view_proj,
            view_index,
            render_phase_mask,
            debug_name,
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        self.eye_position
    }

    pub fn view_proj(&self) -> Mat4 {
        self.view_proj
    }

    pub fn view_index(&self) -> RenderViewIndex {
        self.view_index
    }

    pub fn debug_name(&self) -> &str {
        &self.debug_name
    }

    pub fn phase_is_relevant<RenderPhaseT: RenderPhase>(&self) -> bool {
        self.render_phase_mask.is_included::<RenderPhaseT>()
    }

    pub fn render_phase_mask(&self) -> RenderPhaseMask {
        self.render_phase_mask
    }
}
