use glam::{Mat4, Vec3};
use crate::{RenderPhase, RenderPhaseIndex};
use crate::registry::{RenderPhaseMaskInnerType, MAX_RENDER_PHASE_COUNT};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

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
        self.is_included_index(RenderPhaseT::render_phase_index())
    }

    pub fn is_included_index(
        &self,
        index: RenderPhaseIndex,
    ) -> bool {
        // If this asserts, a render phase was not registered
        assert!(index < MAX_RENDER_PHASE_COUNT);
        (self.0 & 1 << index) != 0
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
        view: Mat4,
        proj: Mat4,
        render_phase_mask: RenderPhaseMask,
        debug_name: String,
    ) -> RenderView {
        let view_index = self.view_count.fetch_add(1, Ordering::Release);
        RenderView::new(
            view_index,
            eye_position,
            view,
            proj,
            render_phase_mask,
            debug_name,
        )
    }

    pub fn view_count(&self) -> RenderViewCount {
        self.view_count.load(Ordering::Acquire)
    }
}

////////////////// Views //////////////////
pub struct RenderViewInner {
    eye_position: Vec3,
    view: Mat4,
    proj: Mat4,
    view_proj: Mat4,
    view_dir: Vec3,
    view_index: RenderViewIndex,
    render_phase_mask: RenderPhaseMask,
    debug_name: String,
}

#[derive(Clone)]
pub struct RenderView {
    inner: Arc<RenderViewInner>,
}

impl RenderView {
    pub fn new(
        view_index: RenderViewIndex,
        eye_position: Vec3,
        view: Mat4,
        proj: Mat4,
        render_phase_mask: RenderPhaseMask,
        debug_name: String,
    ) -> RenderView {
        let view_dir =
            glam::Vec3::new(view.x_axis().z(), view.y_axis().z(), view.z_axis().z()) * -1.0;

        log::trace!("Allocate view {} {}", debug_name, view_index);
        let inner = RenderViewInner {
            eye_position,
            view,
            proj,
            view_proj: proj * view,
            view_dir,
            view_index,
            render_phase_mask,
            debug_name,
        };

        RenderView {
            inner: Arc::new(inner),
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        self.inner.eye_position
    }

    pub fn view_dir(&self) -> Vec3 {
        self.inner.view_dir
    }

    pub fn view_matrix(&self) -> Mat4 {
        self.inner.view
    }

    pub fn projection_matrix(&self) -> Mat4 {
        self.inner.proj
    }

    pub fn view_proj(&self) -> Mat4 {
        self.inner.view_proj
    }

    pub fn view_index(&self) -> RenderViewIndex {
        self.inner.view_index
    }

    pub fn debug_name(&self) -> &str {
        &self.inner.debug_name
    }

    pub fn phase_is_relevant<RenderPhaseT: RenderPhase>(&self) -> bool {
        self.inner.render_phase_mask.is_included::<RenderPhaseT>()
    }

    pub fn phase_index_is_relevant(
        &self,
        phase_index: RenderPhaseIndex,
    ) -> bool {
        self.inner.render_phase_mask.is_included_index(phase_index)
    }

    pub fn render_phase_mask(&self) -> RenderPhaseMask {
        self.inner.render_phase_mask
    }
}
