use demo::assets::anim::BlenderAnimImporter;
use demo::assets::font::FontImporter;
use demo::assets::mesh::{
    BlenderMaterialImporter, BlenderMeshImporter, BlenderModelImporter, BlenderPrefabImporter,
    GltfImporter,
};
use demo::daemon_args::AssetDaemonArgs;
use distill::daemon::AssetDaemon;
use distill_cli::Command;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Clone, PartialEq)]
pub enum CliCommandArgs {
    HostDaemon,
    Pack { path: PathBuf },
}

#[derive(StructOpt, Debug, Clone)]
pub struct CliArgs {
    #[structopt(subcommand)]
    cmd: CliCommandArgs,

    // Pack assets into a single pack file at the given path
    //#[structopt(name = "pack", long, parse(from_os_str))]
    //pub pack: Option<PathBuf>,

    // Host the daemon for other processes to pull from
    //#[structopt(name = "pack", long)]
    //pub host_daemon: bool,

    // Assume the daemon is running externally
    #[structopt(name = "external-daemon", long)]
    pub external_daemon: bool,

    // Extra args for the daemon
    #[structopt(flatten)]
    pub daemon_args: AssetDaemonArgs,
}

fn create_daemon(args: &CliArgs) -> AssetDaemon {
    rafx::assets::distill_impl::default_daemon()
        .with_db_path(&args.daemon_args.db_dir)
        .with_address(args.daemon_args.address)
        .with_asset_dirs(args.daemon_args.asset_dirs.clone())
        .with_importer("ttf", FontImporter)
        .with_importer("gltf", GltfImporter)
        .with_importer("glb", GltfImporter)
        .with_importer("blender_material", BlenderMaterialImporter)
        .with_importer("blender_model", BlenderModelImporter)
        .with_importer("blender_mesh", BlenderMeshImporter)
        .with_importer("blender_prefab", BlenderPrefabImporter)
        .with_importer("blender_anim", BlenderAnimImporter)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = CliArgs::from_args();
    if args.external_daemon && args.cmd == CliCommandArgs::HostDaemon {
        Err("external-daemon and host-daemon args are incompatible".into())
    } else if args.cmd == CliCommandArgs::HostDaemon {
        let asset_daemon = create_daemon(&args);

        // Spawn the daemon in a background thread.
        std::thread::spawn(move || {
            asset_daemon.run();
        });

        // Spin indefinitely
        log::info!("Daemon started, used ctrl-C to terminate");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    } else {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let local = tokio::task::LocalSet::new();
        runtime.block_on(local.run_until(async_main(args)))
    }
}

async fn async_main(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn the daemon in a background thread. This could be a different process, but
    // for simplicity we'll launch it here.
    if !args.external_daemon {
        let asset_daemon = create_daemon(&args);
        std::thread::spawn(move || {
            asset_daemon.run();
        });

        // Give the daemon some time to open the socket
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    match args.cmd {
        CliCommandArgs::HostDaemon => unreachable!(),
        CliCommandArgs::Pack { path } => {
            let context = distill_cli::create_context().await?;
            let cmd_pack = distill_cli::CmdPack;
            cmd_pack
                .run(&context, vec![&path.to_string_lossy()])
                .await?;
            Ok(())
        }
    }
}
