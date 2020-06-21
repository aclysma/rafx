
mod assets_init;
pub use assets_init::init_renderer_assets;
pub use assets_init::update_renderer_assets;
pub use assets_init::destroy_renderer_assets;

pub use renderer_base::time;

pub mod asset_resource;
pub mod asset_storage;
pub mod pipeline;
pub mod image_utils;
pub mod resource_managers;
pub mod push_buffer;
pub mod pipeline_description;
