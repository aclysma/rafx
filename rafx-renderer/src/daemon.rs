use rafx_assets::distill::loader::storage::DefaultIndirectionResolver;
use rafx_assets::distill::loader::{Loader, PackfileReader, RpcIO};
use rafx_assets::distill_impl::AssetResource;
use std::net::SocketAddr;
use std::path::PathBuf;

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

pub fn init_distill_daemon(connect_string: String) -> AssetResource {
    let rpc_loader = RpcIO::new(connect_string).unwrap();
    let loader = Loader::new(Box::new(rpc_loader));
    let resolver = Box::new(DefaultIndirectionResolver);
    AssetResource::new(loader, resolver)
}

pub fn init_distill_packfile(pack_file: &std::path::Path) -> AssetResource {
    let packfile = std::fs::File::open(pack_file).unwrap();
    let packfile_loader = PackfileReader::new(packfile).unwrap();
    let loader = Loader::new(Box::new(packfile_loader));
    let resolver = Box::new(DefaultIndirectionResolver);
    AssetResource::new(loader, resolver)
}
