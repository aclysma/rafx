//! Provides integration with the [`distill`](https://github.com/amethyst/distill) asset
//! pipeline

pub mod assets;

pub use assets::*;

/// Contains some distill-related helpers. They are optional and end-users can provide their own.
pub mod distill_impl;

mod push_buffer;
pub use push_buffer::PushBuffer;
pub use push_buffer::PushBufferResult;
pub use push_buffer::PushBufferSizeCalculator;

mod resource_loader;
pub use resource_loader::ResourceLoader;

pub use distill;

pub mod schema;

mod hydrate_impl;
mod resource_loader_hydrate;

use std::path::PathBuf;

pub fn schema_def_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/schema"))
}
pub use hydrate_impl::register_default_hydrate_plugins;
