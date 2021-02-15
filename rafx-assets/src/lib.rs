//! Provides integration with the [`distill`](https://github.com/amethyst/distill) asset
//! pipeline

pub mod assets;
pub use assets::*;

/// Contains some distill-related helpers. They are optional and end-users can provide their own.
pub mod distill_impl;

pub mod buffer_upload;
pub mod image_upload;

pub mod gpu_image_data;
pub use gpu_image_data::GpuImageData;
pub use gpu_image_data::GpuImageDataColorSpace;
pub use gpu_image_data::GpuImageDataLayer;
pub use gpu_image_data::GpuImageDataMipLevel;

pub mod push_buffer;

mod resource_loader;
pub use resource_loader::ResourceLoader;

pub use distill;
