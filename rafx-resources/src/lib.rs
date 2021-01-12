mod resources;
pub use resources::*;

pub use ash;

pub mod graph;

pub type RafxResult<T> = rafx_api::RafxResult<T>;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
