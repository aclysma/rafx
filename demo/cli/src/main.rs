use atelier_cli::Command;
use demo::daemon;
use demo::daemon::AssetDaemonArgs;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone)]
pub struct CliArgs {
    //
    // For one file at a time
    //
    #[structopt(name = "pack", long, parse(from_os_str))]
    pub pack: Option<PathBuf>,

    #[structopt(flatten)]
    pub daemon_args: AssetDaemonArgs,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let local = tokio::task::LocalSet::new();
    runtime.block_on(local.run_until(async_main()))
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = CliArgs::from_args();

    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    let daemon_args = args.daemon_args.clone().into();
    std::thread::spawn(move || {
        daemon::run(daemon_args);
    });

    // Give the daemon some time to open the socket
    tokio::time::delay_for(tokio::time::Duration::from_secs(1)).await;

    if let Some(path) = &args.pack {
        let context = atelier_cli::create_context().await?;
        let cmd_pack = atelier_cli::CmdPack;
        cmd_pack
            .run(&context, vec![&path.to_string_lossy()])
            .await?;
        Ok(())
    } else {
        Err("No packfile specified".into())
    }
}
