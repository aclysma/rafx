#[derive(Clone)]
pub struct MeshAdvRenderOptions {
    pub show_surfaces: bool,
    pub show_shadows: bool,
    pub enable_lighting: bool,
    pub ambient_light: glam::Vec3,
    pub use_clustered_lighting: bool,
}

impl Default for MeshAdvRenderOptions {
    fn default() -> Self {
        MeshAdvRenderOptions {
            show_surfaces: true,
            show_shadows: true,
            enable_lighting: true,
            ambient_light: glam::Vec3::ZERO,
            use_clustered_lighting: true,
        }
    }
}
