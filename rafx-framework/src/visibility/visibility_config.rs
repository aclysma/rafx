#[derive(Copy, Clone, Debug)]
pub struct VisibilityConfig {
    pub enable_visibility_update: bool,
}

impl Default for VisibilityConfig {
    fn default() -> Self {
        VisibilityConfig {
            enable_visibility_update: true,
        }
    }
}
