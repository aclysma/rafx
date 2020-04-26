use atelier_assets::loader::{
    crossbeam_channel::Sender,
    handle::{AssetHandle, RefOp, TypedAssetStorage},
    AssetLoadOp, AssetStorage, AssetTypeId, LoadHandle, LoaderInfoProvider, TypeUuid,
};
use mopa::{mopafy, Any};
use std::{sync::Mutex, collections::HashMap, error::Error, sync::Arc};

use atelier_assets::importer as atelier_importer;
use atelier_assets::loader as atelier_loader;

pub struct GenericAssetStorage {
    storage: Mutex<HashMap<AssetTypeId, Box<dyn TypedStorage>>>,
    refop_sender: Arc<Sender<RefOp>>,
}

impl GenericAssetStorage {
    pub fn new(refop_sender: Arc<Sender<RefOp>>) -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
            refop_sender,
        }
    }

    pub fn add_storage<T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send>(&self) {
        let mut storages = self.storage.lock().unwrap();
        storages.insert(
            AssetTypeId(T::UUID),
            Box::new(Storage::<T>::new(self.refop_sender.clone())),
        );
    }
}

struct AssetState<A> {
    version: u32,
    asset: A,
}
pub struct Storage<A: TypeUuid> {
    refop_sender: Arc<Sender<RefOp>>,
    assets: HashMap<LoadHandle, AssetState<A>>,
    uncommitted: HashMap<LoadHandle, AssetState<A>>,
}
impl<A: TypeUuid> Storage<A> {
    fn new(sender: Arc<Sender<RefOp>>) -> Self {
        Self {
            refop_sender: sender,
            assets: HashMap::new(),
            uncommitted: HashMap::new(),
        }
    }
    fn get<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<&A> {
        self.assets.get(&handle.load_handle()).map(|a| &a.asset)
    }
    fn get_version<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<u32> {
        self.assets.get(&handle.load_handle()).map(|a| a.version)
    }
    fn get_asset_with_version<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<(&A, u32)> {
        self.assets
            .get(&handle.load_handle())
            .map(|a| (&a.asset, a.version))
    }
}
impl<A: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send> TypedAssetStorage<A>
    for GenericAssetStorage
{
    fn get<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<&A> {
        {
            let x = self
                .storage
                .lock()
                .unwrap()
                .get(&AssetTypeId(A::UUID))
                .expect("unknown asset type")
                .as_ref()
                .type_name();
        }

        // This transmute can probably be unsound, but I don't have the energy to fix it right now
        unsafe {
            std::mem::transmute(
                self.storage
                    .lock()
                    .unwrap()
                    .get(&AssetTypeId(A::UUID))
                    .expect("unknown asset type")
                    .as_ref()
                    .downcast_ref::<Storage<A>>()
                    .expect("failed to downcast")
                    .get(handle),
            )
        }
    }
    fn get_version<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<u32> {
        self.storage
            .lock()
            .unwrap()
            .get(&AssetTypeId(A::UUID))
            .expect("unknown asset type")
            .as_ref()
            .downcast_ref::<Storage<A>>()
            .expect("failed to downcast")
            .get_version(handle)
    }
    fn get_asset_with_version<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<(&A, u32)> {
        // This transmute can probably be unsound, but I don't have the energy to fix it right now
        unsafe {
            std::mem::transmute(
                self.storage
                    .lock()
                    .unwrap()
                    .get(&AssetTypeId(A::UUID))
                    .expect("unknown asset type")
                    .as_ref()
                    .downcast_ref::<Storage<A>>()
                    .expect("failed to downcast")
                    .get_asset_with_version(handle),
            )
        }
    }
}
pub trait TypedStorage: Any + Send {
    fn update_asset(
        &mut self,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error>>;
    fn commit_asset_version(
        &mut self,
        handle: LoadHandle,
        version: u32,
    );
    fn free(
        &mut self,
        handle: LoadHandle,
    );

    fn type_name(&self) -> &'static str;
}
mopafy!(TypedStorage);
impl<A: for<'a> serde::Deserialize<'a> + 'static + TypeUuid + Send> TypedStorage for Storage<A> {
    fn update_asset(
        &mut self,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error>> {
        // To enable automatic serde of Handle, we need to set up a SerdeContext with a RefOp sender
        let asset = atelier_loader::handle::SerdeContext::with_sync(
            loader_info,
            self.refop_sender.clone(),
            || bincode::deserialize::<A>(data),
        )?;
        self.uncommitted
            .insert(load_handle, AssetState { asset, version });
        log::info!("{} bytes loaded for {:?}", data.len(), load_handle);
        // The loading process could be async, in which case you can delay
        // calling `load_op.complete` as it should only be done when the asset is usable.
        load_op.complete();
        Ok(())
    }
    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        _version: u32,
    ) {
        // The commit step is done after an asset load has completed.
        // It exists to avoid frames where an asset that was loaded is unloaded, which
        // could happen when hot reloading. To support this case, you must support having multiple
        // versions of an asset loaded at the same time.
        self.assets.insert(
            load_handle,
            self.uncommitted
                .remove(&load_handle)
                .expect("asset not present when committing"),
        );
        log::info!("Commit {:?}", load_handle);
    }
    fn free(
        &mut self,
        load_handle: LoadHandle,
    ) {
        self.assets.remove(&load_handle);
        log::info!("Free {:?}", load_handle);
    }

    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}

// Untyped implementation of AssetStorage that finds the asset_type's storage and forwards the call
impl AssetStorage for GenericAssetStorage {
    fn update_asset(
        &self,
        loader_info: &dyn LoaderInfoProvider,
        asset_type_id: &AssetTypeId,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error>> {
        self.storage
            .lock()
            .unwrap()
            .get_mut(asset_type_id)
            .expect("unknown asset type")
            .update_asset(loader_info, data, load_handle, load_op, version)
    }
    fn commit_asset_version(
        &self,
        asset_type: &AssetTypeId,
        load_handle: LoadHandle,
        version: u32,
    ) {
        self.storage
            .lock()
            .unwrap()
            .get_mut(asset_type)
            .expect("unknown asset type")
            .commit_asset_version(load_handle, version)
    }
    fn free(
        &self,
        asset_type_id: &AssetTypeId,
        load_handle: LoadHandle,
    ) {
        self.storage
            .lock()
            .unwrap()
            .get_mut(asset_type_id)
            .expect("unknown asset type")
            .free(load_handle)
    }
}
