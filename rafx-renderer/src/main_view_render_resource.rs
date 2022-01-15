use glam::Mat4;
use rafx_framework::render_features::RenderView;

pub struct PreviousMainViewInfo {
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
}

#[derive(Default)]
pub struct MainViewRenderResource {
    pub main_view: Option<RenderView>,
    pub previous_main_view_info: Option<PreviousMainViewInfo>,
}
