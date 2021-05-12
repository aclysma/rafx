use glam::{Mat4, Vec3};
use rafx_api::RafxExtents2D;
use rafx_framework::render_features::{RenderFeatureMask, RenderPhaseMask, RenderViewDepthRange};
use rafx_framework::visibility::ViewFrustumArc;

// Very bare-bones for now, in the future this could support multiple windows, multiple viewports
// per window, and some method for configuring the graph that's being drawn (maybe the graph
// is provided some metadata like a string)

#[derive(Clone)]
pub struct RenderViewMeta {
    pub view_frustum: ViewFrustumArc,
    pub eye_position: Vec3,
    pub view: Mat4,
    pub proj: Mat4,
    pub depth_range: RenderViewDepthRange,
    pub render_phase_mask: RenderPhaseMask,
    pub render_feature_mask: RenderFeatureMask,
    pub debug_name: String,
}

#[derive(Default)]
pub struct ViewportsResource {
    pub main_window_size: RafxExtents2D,
    pub main_view_meta: Option<RenderViewMeta>,
}
