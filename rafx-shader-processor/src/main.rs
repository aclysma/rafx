use log::LevelFilter;
use structopt::StructOpt;

use rafx_shader_processor::*;

fn main() -> Result<(), String> {
    let args = ShaderProcessorArgs::from_args();

    // Setup logging
    let level = if args.trace {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(level)
        .init();

    if let Err(e) = run(&args) {
        eprintln!("{}", e.to_string());
        Err("Shader processor failed".to_string())
    } else {
        Ok(())
    }
}
