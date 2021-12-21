#[derive(Clone)]
pub struct MeshBasicRenderOptions {
    pub show_surfaces: bool,
    pub show_shadows: bool,
    pub enable_lighting: bool,
    pub ambient_light: glam::Vec3,
}

impl Default for MeshBasicRenderOptions {
    fn default() -> Self {
        MeshBasicRenderOptions {
            show_surfaces: true,
            show_shadows: true,
            enable_lighting: true,
            ambient_light: glam::Vec3::ZERO,
        }
    }
}
