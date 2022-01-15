// This is user-facing choices that don't change much frame-to-frame
#[derive(Clone)]
pub struct MeshAdvRenderOptions {
    pub show_surfaces: bool,
    pub show_shadows: bool,
    pub enable_lighting: bool,
    pub ambient_light: glam::Vec3,
    pub ndf_filter_amount: f32,
    pub use_clustered_lighting: bool,
}

impl Default for MeshAdvRenderOptions {
    fn default() -> Self {
        MeshAdvRenderOptions {
            show_surfaces: true,
            show_shadows: true,
            enable_lighting: true,
            ambient_light: glam::Vec3::ZERO,
            ndf_filter_amount: 1.0,
            use_clustered_lighting: true,
        }
    }
}

// This is state that's updated by the pipeline (pipeline pushes data to the feature plugin, not
// the other way around)
pub struct MeshAdvRenderPipelineState {
    pub jitter_amount: glam::Vec2,
    pub forward_pass_mip_bias: f32,
}

impl Default for MeshAdvRenderPipelineState {
    fn default() -> Self {
        MeshAdvRenderPipelineState {
            jitter_amount: glam::Vec2::ZERO,
            forward_pass_mip_bias: 0.0,
        }
    }
}
