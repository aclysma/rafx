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
use hydrate_pipeline::AssetPluginRegistry;
use std::path::PathBuf;

pub fn schema_def_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/schema"))
}

pub fn register_default_hydrate_plugins(
    mut plugin_registry: AssetPluginRegistry
) -> AssetPluginRegistry {
    plugin_registry = plugin_registry
        .register_plugin::<LdtkAssetPlugin>()
        .register_plugin::<FontAssetPlugin>()
        .register_plugin::<BlenderMaterialAssetPlugin>()
        .register_plugin::<BlenderMeshAssetPlugin>()
        .register_plugin::<BlenderModelAssetPlugin>()
        .register_plugin::<BlenderPrefabAssetPlugin>()
        .register_plugin::<BlenderAnimAssetPlugin>()
        .register_plugin::<GltfAssetPlugin>()
        .register_plugin::<MeshAdvAssetPlugin>();

    plugin_registry
}
