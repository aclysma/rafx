use std::{
    net::{AddrParseError, SocketAddr},
    path::PathBuf,
};

use atelier_assets::daemon::AssetDaemon;
use structopt::StructOpt;

/// Parameters to the asset daemon.
///
/// # Examples
///
/// ```bash
/// asset_daemon --db .assets_db --address "127.0.0.1:9999" assets
/// ```
#[derive(StructOpt)]
pub struct AssetDaemonOpt {
    /// Path to the asset metadata database directory.
    #[structopt(name = "db", long, parse(from_os_str), default_value = ".assets_db")]
    pub db_dir: PathBuf,
    /// Socket address for the daemon to listen for connections, e.g. "127.0.0.1:9999".
    #[structopt(
    short,
    long,
    parse(try_from_str = parse_socket_addr),
    default_value = "127.0.0.1:9999"
    )]
    pub address: SocketAddr,
    /// Directories to watch for assets.
    #[structopt(parse(from_os_str), default_value = "assets")]
    pub asset_dirs: Vec<PathBuf>,
}

/// Parses a string as a socket address.
fn parse_socket_addr(s: &str) -> std::result::Result<SocketAddr, AddrParseError> {
    s.parse()
}

pub fn run() {
    let opt = AssetDaemonOpt::from_args();

    AssetDaemon::default()
        .with_importer("pipeline", renderer::assets::PipelineImporter)
        .with_importer("renderpass", renderer::assets::RenderpassImporter)
        .with_importer("sampler", renderer::assets::SamplerImporter)
        .with_importer("material", renderer::assets::MaterialImporter)
        .with_importer(
            "materialinstance",
            renderer::assets::MaterialInstanceImporter,
        )
        .with_importer("spv", renderer::assets::ShaderImporterSpv)
        .with_importer("shader", renderer::assets::ShaderImporterCooked)
        .with_importer("png", renderer::assets::ImageImporter)
        .with_importer("jpg", renderer::assets::ImageImporter)
        .with_importer("jpeg", renderer::assets::ImageImporter)
        .with_importer("tga", renderer::assets::ImageImporter)
        .with_importer("bmp", renderer::assets::ImageImporter)
        .with_importer("gltf", crate::assets::gltf::GltfImporter)
        .with_importer("glb", crate::assets::gltf::GltfImporter)
        .with_db_path(opt.db_dir)
        .with_address(opt.address)
        .with_asset_dirs(opt.asset_dirs)
        .run();
}
