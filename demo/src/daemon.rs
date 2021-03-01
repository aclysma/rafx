use std::{
    net::{AddrParseError, SocketAddr},
    path::PathBuf,
};

use structopt::StructOpt;

/// Parameters to the asset daemon.
///
/// # Examples
///
/// ```bash
/// asset_daemon --db .assets_db --address "127.0.0.1:9999" assets
/// ```
#[derive(StructOpt, Debug, Clone)]
pub struct AssetDaemonArgs {
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

impl Into<AssetDaemonOpt> for AssetDaemonArgs {
    fn into(self) -> AssetDaemonOpt {
        AssetDaemonOpt {
            db_dir: self.db_dir,
            address: self.address,
            asset_dirs: self.asset_dirs,
        }
    }
}

pub struct AssetDaemonOpt {
    pub db_dir: PathBuf,
    pub address: SocketAddr,
    pub asset_dirs: Vec<PathBuf>,
}

impl Default for AssetDaemonOpt {
    fn default() -> Self {
        AssetDaemonOpt {
            db_dir: ".assets_db".into(),
            address: "127.0.0.1:9999".parse().unwrap(),
            asset_dirs: vec!["assets".into()],
        }
    }
}

/// Parses a string as a socket address.
fn parse_socket_addr(s: &str) -> std::result::Result<SocketAddr, AddrParseError> {
    s.parse()
}

pub fn run(opt: AssetDaemonOpt) {
    rafx::assets::distill_impl::default_daemon()
        .with_importer("basis", rafx::assets::BasisImageImporter)
        .with_importer("gltf", crate::assets::gltf::GltfImporter)
        .with_importer("glb", crate::assets::gltf::GltfImporter)
        .with_importer("ttf", crate::assets::font::FontImporter)
        .with_db_path(opt.db_dir)
        .with_address(opt.address)
        .with_asset_dirs(opt.asset_dirs)
        .run();
}
