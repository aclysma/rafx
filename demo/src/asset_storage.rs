use atelier_assets::loader::{
    crossbeam_channel::Sender,
    handle::{AssetHandle, RefOp, TypedAssetStorage},
    storage::{AssetLoadOp, AssetStorage, LoaderInfoProvider, IndirectionTable},
    AssetTypeId, LoadHandle,
};
use mopa::{mopafy, Any};
use std::{sync::Mutex, collections::HashMap, error::Error};

use atelier_assets::core::{AssetUuid, type_uuid::TypeUuid};
use crossbeam_channel::Receiver;
use atelier_assets::loader::handle::SerdeContext;
use std::marker::PhantomData;

// Used to dynamic dispatch into a storage, supports checked downcasting
pub trait DynAssetStorage: Any + Send {
    fn update_asset(
        &mut self,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error + Send + 'static>>;
    fn commit_asset_version(
        &mut self,
        handle: LoadHandle,
        version: u32,
    );
    fn free(
        &mut self,
        handle: LoadHandle,
        version: u32,
    );

    fn type_name(&self) -> &'static str;
}

mopafy!(DynAssetStorage);

pub struct AssetStorageSetInner {
    storage: HashMap<AssetTypeId, Box<dyn DynAssetStorage>>,
    asset_data_type_id_mapping: HashMap<AssetTypeId, AssetTypeId>,
    refop_sender: Sender<RefOp>,
    indirection_table: IndirectionTable,
}

// Contains a storage per asset type
pub struct AssetStorageSet {
    inner: Mutex<AssetStorageSetInner>,
}

impl AssetStorageSet {
    pub fn new(
        refop_sender: Sender<RefOp>,
        indirection_table: IndirectionTable,
    ) -> Self {
        let inner = AssetStorageSetInner {
            storage: Default::default(),
            asset_data_type_id_mapping: Default::default(),
            refop_sender,
            indirection_table,
        };

        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn add_storage<T>(&self)
    where
        T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send,
    {
        let mut inner = self.inner.lock().unwrap();
        let indirection_table = inner.indirection_table.clone();
        let refop_sender = inner.refop_sender.clone();
        inner
            .asset_data_type_id_mapping
            .insert(AssetTypeId(T::UUID), AssetTypeId(T::UUID));
        inner.storage.insert(
            AssetTypeId(T::UUID),
            Box::new(Storage::<T>::new(
                refop_sender,
                Box::new(DefaultAssetLoader::default()),
                indirection_table,
            )),
        );
    }

    pub fn add_storage_with_loader<AssetDataT, AssetT, LoaderT>(
        &self,
        loader: Box<LoaderT>,
    ) where
        AssetDataT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static,
        AssetT: TypeUuid + 'static + Send,
        LoaderT: DynAssetLoader<AssetT> + 'static,
    {
        let mut inner = self.inner.lock().unwrap();
        let indirection_table = inner.indirection_table.clone();
        let refop_sender = inner.refop_sender.clone();
        inner
            .asset_data_type_id_mapping
            .insert(AssetTypeId(AssetDataT::UUID), AssetTypeId(AssetT::UUID));
        inner.storage.insert(
            AssetTypeId(AssetT::UUID),
            Box::new(Storage::<AssetT>::new(
                refop_sender,
                loader,
                indirection_table,
            )),
        );
    }
}

// Implement atelier's AssetStorage - an untyped trait that finds the asset_type's storage and
// forwards the call
impl AssetStorage for AssetStorageSet {
    fn update_asset(
        &self,
        loader_info: &dyn LoaderInfoProvider,
        asset_data_type_id: &AssetTypeId,
        data: Vec<u8>,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error + Send + 'static>> {
        let mut inner = self.inner.lock().unwrap();

        let asset_type_id = *inner
            .asset_data_type_id_mapping
            .get(asset_data_type_id)
            .expect("unknown asset data type");

        inner
            .storage
            .get_mut(&asset_type_id)
            .expect("unknown asset type")
            .update_asset(loader_info, &data, load_handle, load_op, version)
    }

    fn commit_asset_version(
        &self,
        asset_data_type_id: &AssetTypeId,
        load_handle: LoadHandle,
        version: u32,
    ) {
        let mut inner = self.inner.lock().unwrap();

        let asset_type_id = *inner
            .asset_data_type_id_mapping
            .get(asset_data_type_id)
            .expect("unknown asset data type");

        inner
            .storage
            .get_mut(&asset_type_id)
            .expect("unknown asset type")
            .commit_asset_version(load_handle, version)
    }

    fn free(
        &self,
        asset_data_type_id: &AssetTypeId,
        load_handle: LoadHandle,
        version: u32,
    ) {
        let mut inner = self.inner.lock().unwrap();

        let asset_type_id = *inner
            .asset_data_type_id_mapping
            .get(asset_data_type_id)
            .expect("unknown asset data type");

        inner
            .storage
            .get_mut(&asset_type_id)
            .expect("unknown asset type")
            .free(load_handle, version)
    }
}

// Implement atelier's TypedAssetStorage - a typed trait that finds the asset_type's storage and
// forwards the call
impl<A: TypeUuid + 'static + Send> TypedAssetStorage<A> for AssetStorageSet {
    fn get<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<&A> {
        // This transmute can probably be unsound, but I don't have the energy to fix it right now
        unsafe {
            std::mem::transmute(
                self.inner
                    .lock()
                    .unwrap()
                    .storage
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
        self.inner
            .lock()
            .unwrap()
            .storage
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
                self.inner
                    .lock()
                    .unwrap()
                    .storage
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

// Loaders can return immediately by value, or later by returning a channel
pub enum UpdateAssetResult<AssetT>
where
    AssetT: Send,
{
    Result(AssetT),
    AsyncResult(Receiver<AssetT>),
}

// Implements loading logic (i.e. turning bytes into an asset. The asset may contain runtime-only
// data and may be created asynchronously
pub trait DynAssetLoader<AssetT>: Send
where
    AssetT: TypeUuid + 'static + Send,
{
    fn update_asset(
        &mut self,
        refop_sender: &Sender<RefOp>,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<UpdateAssetResult<AssetT>, Box<dyn Error + Send + 'static>>;

    fn commit_asset_version(
        &mut self,
        handle: LoadHandle,
        version: u32,
    );

    fn free(
        &mut self,
        handle: LoadHandle,
    );
}

// A simple loader that just deserializes data
struct DefaultAssetLoader<AssetDataT>
where
    AssetDataT: TypeUuid + Send + for<'a> serde::Deserialize<'a> + 'static,
{
    phantom_data: PhantomData<AssetDataT>,
}

impl<AssetDataT> Default for DefaultAssetLoader<AssetDataT>
where
    AssetDataT: TypeUuid + Send + for<'a> serde::Deserialize<'a> + 'static,
{
    fn default() -> Self {
        DefaultAssetLoader {
            phantom_data: Default::default(),
        }
    }
}

impl<AssetDataT> DynAssetLoader<AssetDataT> for DefaultAssetLoader<AssetDataT>
where
    AssetDataT: TypeUuid + Send + for<'a> serde::Deserialize<'a> + 'static,
{
    fn update_asset(
        &mut self,
        refop_sender: &Sender<RefOp>,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        _load_handle: LoadHandle,
        load_op: AssetLoadOp,
        _version: u32,
    ) -> Result<UpdateAssetResult<AssetDataT>, Box<dyn Error + Send + 'static>> {
        let asset = futures_lite::future::block_on(SerdeContext::with(
            loader_info,
            refop_sender.clone(),
            async {
                bincode::deserialize::<AssetDataT>(data)
                    // Coerce into boxed error
                    .map_err(|x| -> Box<dyn Error + Send + 'static> { Box::new(x) })
            },
        ))?;

        load_op.complete();
        Ok(UpdateAssetResult::Result(asset))
    }

    fn commit_asset_version(
        &mut self,
        _handle: LoadHandle,
        _version: u32,
    ) {
    }

    fn free(
        &mut self,
        _handle: LoadHandle,
    ) {
    }
}

struct UncommittedAssetState<A: Send> {
    version: u32,
    asset_uuid: AssetUuid,
    result: UpdateAssetResult<A>,
}

struct AssetState<A> {
    version: u32,
    asset_uuid: AssetUuid,
    asset: A,
}

// A strongly typed storage for a single asset type
pub struct Storage<AssetT: TypeUuid + Send> {
    refop_sender: Sender<RefOp>,
    assets: HashMap<LoadHandle, AssetState<AssetT>>,
    uncommitted: HashMap<LoadHandle, UncommittedAssetState<AssetT>>,
    loader: Box<dyn DynAssetLoader<AssetT>>,
    indirection_table: IndirectionTable,
}

fn resolve_load_handle<T: AssetHandle>(
    handle: &T,
    indirection_table: &IndirectionTable,
) -> Option<LoadHandle> {
    if handle.load_handle().is_indirect() {
        indirection_table.resolve(handle.load_handle())
    } else {
        Some(handle.load_handle())
    }
}

impl<AssetT: TypeUuid + Send> Storage<AssetT> {
    fn new(
        sender: Sender<RefOp>,
        loader: Box<dyn DynAssetLoader<AssetT>>,
        indirection_table: IndirectionTable,
    ) -> Self {
        Self {
            refop_sender: sender,
            assets: HashMap::new(),
            uncommitted: HashMap::new(),
            loader,
            indirection_table,
        }
    }
    fn get<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<&AssetT> {
        let load_handle = resolve_load_handle(handle, &self.indirection_table)?;
        self.assets.get(&load_handle).map(|a| &a.asset)
    }
    fn get_version<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<u32> {
        let load_handle = resolve_load_handle(handle, &self.indirection_table)?;
        self.assets.get(&load_handle).map(|a| a.version)
    }
    fn get_asset_with_version<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<(&AssetT, u32)> {
        let load_handle = resolve_load_handle(handle, &self.indirection_table)?;
        self.assets.get(&load_handle).map(|a| (&a.asset, a.version))
    }
}

impl<AssetT: TypeUuid + 'static + Send> DynAssetStorage for Storage<AssetT> {
    fn update_asset(
        &mut self,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error + Send + 'static>> {
        log::trace!(
            "update_asset {} {:?} {:?} {}",
            core::any::type_name::<AssetT>(),
            load_handle,
            loader_info.get_asset_id(load_handle).unwrap(),
            version
        );

        let result = self.loader.update_asset(
            &self.refop_sender,
            loader_info,
            data,
            load_handle,
            load_op,
            version,
        )?;
        let asset_uuid = loader_info.get_asset_id(load_handle).unwrap();

        // Add to list of uncommitted assets
        self.uncommitted.insert(
            load_handle,
            UncommittedAssetState {
                asset_uuid,
                result,
                version,
            },
        );

        Ok(())
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        version: u32,
    ) {
        // Remove from the uncommitted list
        let uncommitted_asset_state = self
            .uncommitted
            .remove(&load_handle)
            .expect("asset not present when committing");

        log::trace!(
            "commit_asset_version {} {:?} {:?} {}",
            core::any::type_name::<AssetT>(),
            load_handle,
            uncommitted_asset_state.asset_uuid,
            version
        );

        let asset_uuid = uncommitted_asset_state.asset_uuid;
        let version = uncommitted_asset_state.version;
        let asset = match uncommitted_asset_state.result {
            UpdateAssetResult::Result(asset) => asset,
            UpdateAssetResult::AsyncResult(rx) => rx
                .recv_timeout(std::time::Duration::from_secs(0))
                .expect("LoadOp committed but result not sent via channel"),
        };

        // If a load handler exists, trigger the commit_asset_version callback
        self.loader.commit_asset_version(load_handle, version);

        let asset_state = AssetState {
            asset,
            asset_uuid,
            version,
        };

        // Commit the result
        self.assets.insert(load_handle, asset_state);
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
        version: u32,
    ) {
        if let Some(asset_state) = self.assets.get(&load_handle) {
            if asset_state.version == version {
                // Remove it from the list of assets
                let asset_state = self.assets.remove(&load_handle).unwrap();

                log::trace!(
                    "free {} {:?} {:?}",
                    core::any::type_name::<AssetT>(),
                    load_handle,
                    asset_state.asset_uuid
                );
                // Trigger the free callback on the load handler, if one exists
                self.loader.free(load_handle);
            }
        }
    }

    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}
