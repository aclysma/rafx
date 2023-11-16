use std::path::PathBuf;
//
// fn schema_def_path() -> PathBuf {
//     PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/schema"))
// }

fn schema_cache_file_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/schema_cache_file.json"
    ))
}

fn asset_id_based_data_source_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/assets_id_based"))
}

fn asset_path_based_data_source_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/assets_path_based"
    ))
}

pub fn import_data_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/import_data"))
}

pub fn build_data_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/build_data"))
}

pub fn job_data_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/data/job_data"))
}

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

    let mut linker = hydrate::model::SchemaLinker::default();

    let mut asset_plugin_registration_helper =
        hydrate::pipeline::AssetPluginRegistrationHelper::new();

    asset_plugin_registration_helper = rafx::assets::register_default_hydrate_plugins(
        asset_plugin_registration_helper,
        &mut linker,
    );

    asset_plugin_registration_helper = rafx_plugins::register_default_hydrate_plugins(
        asset_plugin_registration_helper,
        &mut linker,
    );

    /*
    .register_plugin::<rafx::assets::MaterialAssetPlugin>(&mut linker)
    .register_plugin::<GpuImageAssetPlugin>(&mut linker)
    .register_plugin::<BlenderMaterialAssetPlugin>(&mut linker)
    .register_plugin::<BlenderMeshAssetPlugin>(&mut linker)
    .register_plugin::<MeshAdvAssetPlugin>(&mut linker)
    .register_plugin::<GlslAssetPlugin>(&mut linker)
    .register_plugin::<GltfAssetPlugin>(&mut linker)
    .register_plugin::<SimpleDataAssetPlugin>(&mut linker);
    */

    //TODO: Take a config file
    //TODO: Support N sources using path nodes
    let schema_set = hydrate::editor::DbState::load_schema(
        linker,
        &[
            &rafx::assets::schema_def_path(),
            &rafx_plugins::schema_def_path(),
        ],
        &schema_cache_file_path(),
    );

    let (importer_registry, builder_registry, job_processor_registry) =
        asset_plugin_registration_helper.finish(&schema_set);

    let mut imports_to_queue = Vec::default();
    let mut db_state = hydrate::editor::DbState::load_or_init_empty(
        &schema_set,
        &importer_registry,
        &asset_id_based_data_source_path(),
        &asset_path_based_data_source_path(),
        &schema_cache_file_path(),
        &mut imports_to_queue,
    );

    let mut asset_engine = hydrate::pipeline::AssetEngine::new(
        &schema_set,
        importer_registry,
        builder_registry,
        job_processor_registry,
        &db_state.editor_model,
        import_data_path(),
        job_data_path(),
        build_data_path(),
    );

    for import_to_queue in imports_to_queue {
        //println!("Queueing import operation {:?}", import_to_queue);
        asset_engine.queue_import_operation(
            import_to_queue.requested_importables,
            import_to_queue.importer_id,
            import_to_queue.source_file_path,
            import_to_queue.assets_to_regenerate,
        );
    }

    //Headless
    asset_engine.update(&mut db_state.editor_model);

    hydrate::editor::run(db_state, asset_engine);
}
