//! Provides integration with the [`distill`](https://github.com/amethyst/distill) asset
//! pipeline

pub mod assets;
pub use assets::*;

/// Contains some distill-related helpers. They are optional and end-users can provide their own.
pub mod distill_impl;

pub mod buffer_upload;
pub mod image_upload;

pub mod decoded_image;
pub use decoded_image::DecodedImage;
pub use decoded_image::DecodedImageColorSpace;
pub use decoded_image::DecodedImageMips;

pub mod push_buffer;

mod resource_loader;
pub use resource_loader::ResourceLoader;

pub use distill;
