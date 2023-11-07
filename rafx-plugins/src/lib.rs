pub mod assets;
pub mod components;
pub mod features;
pub mod phases;
pub mod pipelines;

mod schema;
mod shaders;

use crate::assets::anim::BlenderAnimAssetPlugin;
use crate::assets::font::FontAssetPlugin;
use crate::assets::ldtk::LdtkAssetPlugin;
use crate::assets::mesh_adv::{
    BlenderMaterialAssetPlugin, BlenderMeshAssetPlugin, BlenderModelAssetPlugin,
    BlenderPrefabAssetPlugin, GltfAssetPlugin, MeshAdvAssetPlugin,
};
use hydrate_model::{AssetPluginRegistrationHelper, SchemaLinker};
use std::path::PathBuf;

pub fn schema_def_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/schema"))
}

pub fn register_default_hydrate_plugins(
    mut registration_helper: AssetPluginRegistrationHelper,
    schema_linker: &mut SchemaLinker,
) -> AssetPluginRegistrationHelper {
    registration_helper = registration_helper
        .register_plugin::<LdtkAssetPlugin>(schema_linker)
        .register_plugin::<FontAssetPlugin>(schema_linker)
        .register_plugin::<BlenderMaterialAssetPlugin>(schema_linker)
        .register_plugin::<BlenderMeshAssetPlugin>(schema_linker)
        .register_plugin::<BlenderModelAssetPlugin>(schema_linker)
        .register_plugin::<BlenderPrefabAssetPlugin>(schema_linker)
        .register_plugin::<BlenderAnimAssetPlugin>(schema_linker)
        .register_plugin::<GltfAssetPlugin>(schema_linker)
        .register_plugin::<MeshAdvAssetPlugin>(schema_linker);

    registration_helper
}
