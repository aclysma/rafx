//! Lowest level crate of `rafx`. Includes some basic memory management and other utilities

mod decimal;
pub use decimal::DecimalF32;
pub use decimal::DecimalF64;

pub mod slab;

pub mod memory;

pub mod offsetof;

pub mod resource_map;
pub mod resource_ref_map;

pub mod trust_cell;
