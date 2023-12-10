use hydrate::model::{AssetPathCache, EditorModelWithCache};
use hydrate::pipeline::HydrateProjectConfiguration;
use std::path::PathBuf;

mod inspectors;

fn main() {
    #[cfg(feature = "profile-with-tracy")]
    profiling::tracy_client::Client::start();
    #[cfg(feature = "profile-with-tracy")]
    profiling::tracy_client::set_thread_name!("Main Thread");

    // Setup logging
    env_logger::Builder::default()
        .write_style(env_logger::WriteStyle::Always)
        .filter_module("globset", log::LevelFilter::Trace)
        .filter_level(log::LevelFilter::Debug)
        .init();

    let project_configuration = HydrateProjectConfiguration::locate_project_file(&PathBuf::from(
        env!("CARGO_MANIFEST_DIR"),
    ))
    .unwrap();

    let mut asset_plugin_registry = hydrate::pipeline::AssetPluginRegistry::new();
    asset_plugin_registry = rafx::assets::register_default_hydrate_plugins(asset_plugin_registry);
    asset_plugin_registry = rafx_plugins::register_default_hydrate_plugins(asset_plugin_registry);

    let mut editor = hydrate::editor::Editor::new(project_configuration, asset_plugin_registry);

    let schema_set = editor.schema_set().clone();
    inspectors::register_inspectors(&schema_set, editor.inspector_registry_mut());

    editor.run().unwrap()
}
