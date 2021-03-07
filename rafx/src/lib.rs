//! NOTE: **docs.rs may not generate complete documentation for rafx because it does not enable any
//! features in cargo when building the docs.** To generate complete documentation locally, run
//! `cargo doc --no-deps --open` in the root of the crate.
//!
//! **Please see [additional documentation here](https://github.com/aclysma/rafx/blob/master/docs/index.md)**
//!
//! Rafx is a multi-backend renderer that prioritizes performance, flexibility, and productivity. It
//! optionally integrates with the [`distill`](https://github.com/amethyst/distill) asset
//! pipeline to provide workflows and tools suitable for real-world projects with multidisciplinary
//! teams.
//!
//! This crate contains several layers:
//!  * [`rafx_api`]: Low-level graphics API abstraction
//!  * [`rafx_framework`]: Mid-level framework that eases resource management, lifetime handling, and
//!     draw call dispatching
//!  * [`rafx_assets`]: Asset layer that integrates with the
//!     [`distill`](https://github.com/amethyst/distill) asset pipeline
//!      * NOTE: The published version in crates.io does not include rafx-assets as `distill` is not
//!        published yet
//!
//! Rafx also provides tools for building shaders and packing assets.
//!
//! Rafx supports most mainstream platforms via `vulkan` and `metal` backends. Proprietary platforms
//! can be supported by adding an additional backend.

pub use rafx_base as base;

pub use rafx_api as api;

#[cfg(feature = "assets")]
pub use rafx_assets as assets;
#[cfg(feature = "assets")]
pub use rafx_assets::distill;

#[cfg(feature = "framework")]
pub use rafx_framework as framework;
#[cfg(feature = "framework")]
pub use rafx_framework::declare_render_feature;
#[cfg(feature = "framework")]
pub use rafx_framework::declare_render_phase;
#[cfg(feature = "framework")]
pub use rafx_framework::graph;
#[cfg(feature = "framework")]
pub use rafx_framework::nodes;
#[cfg(feature = "framework")]
pub use rafx_framework::visibility;

#[cfg(feature = "renderer")]
pub use rafx_renderer as renderer;
