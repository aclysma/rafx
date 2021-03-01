//! A mid-level framework for rendering that provides tools for resource lifetime management,
//! descriptor set management, materials, renderpass management, and draw call dispatching

mod resources;
pub use resources::*;

pub mod graph;

pub mod nodes;

pub mod visibility;

pub use rafx_api::RafxResult;

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
