use log::LevelFilter;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

use hydrate_codegen::*;

fn do_codegen() -> Result<(), Box<dyn Error>> {
    hydrate_codegen::run(&HydrateCodegenArgs {
        schema_path: PathBuf::from_str(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rafx-assets/schema"
        ))
        .unwrap(),
        included_schema: Default::default(),
        outfile: PathBuf::from_str(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rafx-assets/src/schema_codegen.rs"
        ))
        .unwrap(),
        trace: false,
    })?;

    hydrate_codegen::run(&HydrateCodegenArgs {
        schema_path: PathBuf::from_str(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rafx-plugins/schema"
        ))
        .unwrap(),
        included_schema: vec![PathBuf::from_str(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rafx-assets/schema"
        ))
        .unwrap()],
        outfile: PathBuf::from_str(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rafx-plugins/src/schema_codegen.rs"
        ))
        .unwrap(),
        trace: false,
    })?;

    Ok(())
}

fn main() -> Result<(), String> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(LevelFilter::Info)
        .init();

    if let Err(e) = do_codegen() {
        eprintln!("{}", e.to_string());
        Err("Hydrate codegen failed".to_string())
    } else {
        Ok(())
    }
}
