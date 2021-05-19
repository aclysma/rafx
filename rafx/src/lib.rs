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
pub use rafx_framework::declare_render_feature_flag;
#[cfg(feature = "framework")]
pub use rafx_framework::declare_render_phase;
#[cfg(feature = "framework")]
pub use rafx_framework::graph;
#[cfg(feature = "framework")]
pub use rafx_framework::render_features;
#[cfg(feature = "framework")]
pub use rafx_framework::visibility;
#[cfg(feature = "framework")]
pub mod render_feature_extract_job_predule {
    pub use rafx_framework::render_features::render_features_prelude::*;
    pub use std::sync::Arc;
}
#[cfg(feature = "framework")]
pub mod render_feature_prepare_job_predule {
    pub use rafx_framework::render_features::render_features_prelude::*;
    pub use std::sync::Arc;
}
#[cfg(feature = "framework")]
pub mod render_feature_write_job_prelude {
    pub use rafx_api::RafxResult;
    pub use rafx_framework::render_features::render_features_prelude::*;
    pub use std::sync::Arc;
}
#[cfg(feature = "framework")]
pub mod render_feature_renderer_prelude {
    pub use rafx_api::extra::upload::RafxTransferUpload;
    pub use rafx_api::RafxResult;
    #[cfg(feature = "assets")]
    pub use rafx_assets::distill_impl::AssetResource;
    #[cfg(feature = "assets")]
    pub use rafx_assets::AssetManager;
    pub use rafx_base::resource_map::ResourceMap;
    pub use rafx_framework::render_features::render_features_prelude::*;
    pub use rafx_framework::RenderResources;
    #[cfg(feature = "renderer")]
    pub use rafx_renderer::RenderFeaturePlugin;
    pub use std::sync::Arc;
}
#[cfg(feature = "framework")]
pub mod render_feature_mod_prelude {
    pub use rafx_framework::render_features::{
        RenderFeature, RenderFeatureDebugConstants, RenderFeatureFlag, RenderFeatureFlagIndex,
        RenderFeatureIndex,
    };
    pub use std::convert::TryInto;
}
#[cfg(feature = "framework")]
pub use rafx_visibility;

#[cfg(feature = "renderer")]
pub use rafx_renderer as renderer;
