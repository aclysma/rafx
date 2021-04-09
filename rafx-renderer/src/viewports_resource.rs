use rafx_api::RafxExtents2D;
use rafx_framework::nodes::{RenderFeatureMask, RenderPhaseMask, RenderViewDepthRange};

// Very bare-bones for now, in the future this could support multiple windows, multiple viewports
// per window, and some method for configuring the graph that's being drawn (maybe the graph
// is provided some metadata like a string)

#[derive(Clone)]
pub struct RenderViewMeta {
    pub eye_position: glam::Vec3,
    pub view: glam::Mat4,
    pub proj: glam::Mat4,
    pub depth_range: RenderViewDepthRange,
    pub render_phase_mask: RenderPhaseMask,
    pub render_feature_mask: RenderFeatureMask,
    pub debug_name: String,
}

impl Default for RenderViewMeta {
    fn default() -> Self {
        let eye = glam::Vec3::ZERO;
        let up = glam::Vec3::Z;
        let target = glam::Vec3::Y;

        let view = glam::Mat4::look_at_rh(eye, target, up);
        let proj = glam::Mat4::IDENTITY;

        RenderViewMeta {
            eye_position: eye,
            view,
            proj,
            depth_range: RenderViewDepthRange::new_infinite_reverse(0.1),
            render_phase_mask: RenderPhaseMask::empty(),
            render_feature_mask: RenderFeatureMask::empty(),
            debug_name: "undefined".to_string(),
        }
    }
}

#[derive(Default)]
pub struct ViewportsResource {
    pub main_window_size: RafxExtents2D,
    pub main_view_meta: Option<RenderViewMeta>,
}
