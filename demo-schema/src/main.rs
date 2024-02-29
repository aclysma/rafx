use log::LevelFilter;
use std::path::PathBuf;

fn main() -> Result<(), String> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Info)
        .init();

    hydrate_codegen::run(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        &hydrate_codegen::HydrateCodegenArgs::default(),
    )
    .unwrap();
    Ok(())
}
