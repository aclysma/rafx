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
