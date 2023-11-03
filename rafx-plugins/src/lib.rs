pub mod assets;
pub mod components;
pub mod features;
pub mod phases;
pub mod pipelines;

mod schema;
mod shaders;

use crate::assets::font::FontAssetPlugin;
use hydrate_model::{AssetPluginRegistrationHelper, SchemaLinker};
use std::path::PathBuf;

pub fn schema_def_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/schema"))
}

pub fn register_default_hydrate_plugins(
    mut registration_helper: AssetPluginRegistrationHelper,
    schema_linker: &mut SchemaLinker,
) -> AssetPluginRegistrationHelper {
    use crate::assets::*;

    registration_helper = registration_helper.register_plugin::<FontAssetPlugin>(schema_linker);

    registration_helper
}
