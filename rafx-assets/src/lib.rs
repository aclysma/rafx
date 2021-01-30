//! Provides integration with the [`distill`](https://github.com/amethyst/atelier-assets) asset
//! pipeline

pub mod assets;
pub use assets::*;

pub mod buffer_upload;
pub mod image_upload;

pub mod decoded_image;
pub use decoded_image::DecodedImage;
pub use decoded_image::DecodedImageColorSpace;
pub use decoded_image::DecodedImageMips;

pub mod push_buffer;

mod resource_loader;
pub use resource_loader::ResourceLoader;

pub use atelier_assets;
