#[derive(Clone)]
pub struct MeshRenderOptions {
    pub show_surfaces: bool,
    pub show_shadows: bool,
    pub enable_lighting: bool,
}

impl Default for MeshRenderOptions {
    fn default() -> Self {
        MeshRenderOptions {
            show_surfaces: true,
            show_shadows: true,
            enable_lighting: true,
        }
    }
}
