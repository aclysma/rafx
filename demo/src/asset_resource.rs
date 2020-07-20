use atelier_assets::loader::{handle::RefOp, rpc_loader::RpcLoader, Loader};

use std::sync::Arc;

use type_uuid::TypeUuid;

use atelier_assets::loader as atelier_loader;
use crate::asset_storage::{AssetStorageSet, DynAssetLoader};

// A legion-friendly container for assets storages
pub struct AssetResource {
    loader: RpcLoader,
    storage: AssetStorageSet,
    tx: Arc<atelier_loader::crossbeam_channel::Sender<RefOp>>,
    rx: atelier_loader::crossbeam_channel::Receiver<RefOp>,
}

impl AssetResource {
    fn new(loader: RpcLoader) -> Self {
        let (tx, rx) = atelier_loader::crossbeam_channel::unbounded();
        let tx = Arc::new(tx);
        let storage = AssetStorageSet::new(tx.clone());

        AssetResource {
            loader,
            storage,
            tx,
            rx,
        }
    }
}

impl AssetResource {
    pub fn add_storage<AssetDataT>(&mut self)
    where
        AssetDataT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send,
    {
        self.storage.add_storage::<AssetDataT>();
    }

    pub fn add_storage_with_loader<AssetDataT, AssetT, LoaderT>(
        &mut self,
        loader: Box<LoaderT>,
    ) where
        AssetDataT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static,
        AssetT: TypeUuid + 'static + Send,
        LoaderT: DynAssetLoader<AssetT> + 'static,
    {
        self.storage
            .add_storage_with_loader::<AssetDataT, AssetT, LoaderT>(loader);
    }

    pub fn update(&mut self) {
        atelier_loader::handle::process_ref_ops(&self.loader, &self.rx);
        self.loader
            .process(&self.storage)
            .expect("failed to process loader");
    }

    pub fn loader(&self) -> &RpcLoader {
        &self.loader
    }

    pub fn storage(&self) -> &AssetStorageSet {
        &self.storage
    }

    pub fn tx(&self) -> &Arc<atelier_loader::crossbeam_channel::Sender<RefOp>> {
        &self.tx
    }
}
