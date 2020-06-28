use atelier_assets::loader::{
    crossbeam_channel::Sender,
    handle::{AssetHandle, RefOp, TypedAssetStorage},
    AssetLoadOp, AssetStorage, AssetTypeId, LoadHandle, LoaderInfoProvider, TypeUuid,
};
use mopa::{mopafy, Any};
use std::{sync::Mutex, collections::HashMap, error::Error, sync::Arc};

use atelier_assets::core::AssetUuid;
use renderer::resources::ResourceLoadHandler;
use crossbeam_channel::Receiver;

// Used to dynamic dispatch into a storage, supports checked downcasting
pub trait DynAssetStorage: Any + Send {
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

mopafy!(DynAssetStorage);

// Contains a storage per asset type
#[derive(Default)]
pub struct AssetStorageSetInner {
    storage: HashMap<AssetTypeId, Box<dyn DynAssetStorage>>,
    asset_data_type_id_mapping: HashMap<AssetTypeId, AssetTypeId>,
}

// Contains a storage per asset type
pub struct AssetStorageSet {
    inner: Mutex<AssetStorageSetInner>,
    refop_sender: Arc<Sender<RefOp>>,
}

impl AssetStorageSet {
    pub fn new(refop_sender: Arc<Sender<RefOp>>) -> Self {
        Self {
            inner: Mutex::new(Default::default()),
            refop_sender,
        }
    }

    pub fn add_storage<T>(&self)
    where
        T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send,
    {
        let mut inner = self.inner.lock().unwrap();
        inner.asset_data_type_id_mapping.insert(AssetTypeId(T::UUID), AssetTypeId(T::UUID));
        inner.storage.insert(
            AssetTypeId(T::UUID),
            Box::new(
                Storage::<T>::new(
                    self.refop_sender.clone(),
                    Box::new(DefaultAssetLoader::default())
                )
            ),
        );
    }

    //TODO: This could be redesigned to integrate better with the contents of asset_storage.rs
    // - Currently, we make a Storage<T> with T being the raw asset, and proxy events to the load handler
    // - The load handler goes through load queues and puts the instantiated asset in the resource
    //   manager's asset_lookup
    // - This means the asset storage has a fairly useless asset, and the actually useful struct
    //   (i.e. LoadedMesh vs. MeshAsset) has to be retrieved through a totally different path
    pub fn add_storage_with_load_handler<AssetT, LoadedT, LoaderT>(
        &self,
        loader: Box<LoaderT>,
    ) where
        AssetT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static,
        LoadedT: TypeUuid + 'static + Send,
        LoaderT: DynAssetLoader<LoadedT> + 'static,
    {
        let mut inner = self.inner.lock().unwrap();
        inner.asset_data_type_id_mapping.insert(AssetTypeId(AssetT::UUID), AssetTypeId(LoadedT::UUID));
        inner.storage.insert(
            AssetTypeId(LoadedT::UUID),
            Box::new(Storage::<LoadedT>::new(
                self.refop_sender.clone(),
                loader,
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
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error>> {
        let mut inner = self.inner
            .lock()
            .unwrap();

        let asset_type_id = *inner.asset_data_type_id_mapping
            .get(asset_data_type_id)
            .expect("unknown asset data type");

        inner.storage
            .get_mut(&asset_type_id)
            .expect("unknown asset type")
            .update_asset(loader_info, data, load_handle, load_op, version)
    }

    fn commit_asset_version(
        &self,
        asset_data_type_id: &AssetTypeId,
        load_handle: LoadHandle,
        version: u32,
    ) {
        let mut inner = self.inner
            .lock()
            .unwrap();

        let asset_type_id = *inner.asset_data_type_id_mapping
            .get(asset_data_type_id)
            .expect("unknown asset data type");

        inner.storage
            .get_mut(&asset_type_id)
            .expect("unknown asset type")
            .commit_asset_version(load_handle, version)
    }

    fn free(
        &self,
        asset_data_type_id: &AssetTypeId,
        load_handle: LoadHandle,
    ) {
        let mut inner = self.inner
            .lock()
            .unwrap();

        let asset_type_id = *inner.asset_data_type_id_mapping
            .get(asset_data_type_id)
            .expect("unknown asset data type");

        inner.storage
            .get_mut(&asset_type_id)
            .expect("unknown asset type")
            .free(load_handle)
    }
}

// Implement atelier's TypedAssetStorage - a typed trait that finds the asset_type's storage and
// forwards the call
impl<A: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send> TypedAssetStorage<A>
    for AssetStorageSet
{
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

pub enum UpdateAssetResult<LoadedT>
    where
        LoadedT: Send
{
    Result(LoadedT),
    AsyncResult(Receiver<LoadedT>)
}

// Used to dynamic dispatch into a storage, supports checked downcasting
pub trait DynAssetLoader<LoadedT> : Send
where
    LoadedT: TypeUuid + 'static + Send
{
    fn update_asset(
        &mut self,
        refop_sender: &Arc<Sender<RefOp>>,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<UpdateAssetResult<LoadedT>, Box<dyn Error>>;

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

struct ResourceLoader<AssetT, LoadedT> {
    load_handler: Box<dyn ResourceLoadHandler<AssetT, LoadedT>>
}

use atelier_assets::loader::handle::SerdeContext;
use std::marker::PhantomData;

struct DefaultAssetLoader<AssetT>
    where
        AssetT: TypeUuid + Send + for<'a> serde::Deserialize<'a> + 'static,
{
    phantom_data: PhantomData<AssetT>
}

impl<AssetT> Default for DefaultAssetLoader<AssetT>
    where
        AssetT: TypeUuid + Send + for<'a> serde::Deserialize<'a> + 'static,
{
    fn default() -> Self {
        DefaultAssetLoader {
            phantom_data: Default::default()
        }
    }
}

impl<AssetT> DynAssetLoader<AssetT> for DefaultAssetLoader<AssetT>
    where
        AssetT: TypeUuid + Send + for<'a> serde::Deserialize<'a> + 'static,
{
    fn update_asset(
        &mut self,
        refop_sender: &Arc<Sender<RefOp>>,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        _load_handle: LoadHandle,
        load_op: AssetLoadOp,
        _version: u32
    ) -> Result<UpdateAssetResult<AssetT>, Box<dyn Error>> {

        let asset = SerdeContext::with_sync(
            loader_info,
            refop_sender.clone(),
            || bincode::deserialize::<AssetT>(data),
        )?;

        load_op.complete();
        Ok(UpdateAssetResult::Result(asset))
    }

    fn commit_asset_version(
        &mut self,
        _handle: LoadHandle,
        _version: u32
    ) {

    }

    fn free(
        &mut self,
        _handle: LoadHandle
    ) {

    }
}


struct UncommittedAssetState<A : Send> {
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
pub struct Storage<LoadedT: TypeUuid + Send> {
    refop_sender: Arc<Sender<RefOp>>,
    assets: HashMap<LoadHandle, AssetState<LoadedT>>,
    uncommitted: HashMap<LoadHandle, UncommittedAssetState<LoadedT>>,
    load_handler: Box<dyn DynAssetLoader<LoadedT>>,
}

impl<LoadedT: TypeUuid + Send> Storage<LoadedT> {
    fn new(
        sender: Arc<Sender<RefOp>>,
        loader: Box<dyn DynAssetLoader<LoadedT>>,
    ) -> Self {
        Self {
            refop_sender: sender,
            assets: HashMap::new(),
            uncommitted: HashMap::new(),
            load_handler: loader,
        }
    }
    fn get<T: AssetHandle>(
        &self,
        handle: &T,
    ) -> Option<&LoadedT> {
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
    ) -> Option<(&LoadedT, u32)> {
        self.assets
            .get(&handle.load_handle())
            .map(|a| (&a.asset, a.version))
    }
}

impl<LoadedT: TypeUuid + 'static + Send> DynAssetStorage for Storage<LoadedT> {
    fn update_asset(
        &mut self,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        version: u32,
    ) -> Result<(), Box<dyn Error>> {
        log::trace!(
            "update_asset {} {:?} {:?} {}",
            core::any::type_name::<LoadedT>(),
            load_handle,
            loader_info.get_asset_id(load_handle).unwrap(),
            version
        );

        let result = self.load_handler.update_asset(&self.refop_sender, loader_info, data, load_handle, load_op, version)?;
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
            core::any::type_name::<LoadedT>(),
            load_handle,
            uncommitted_asset_state.asset_uuid,
            version
        );

        let asset_uuid = uncommitted_asset_state.asset_uuid;
        let version = uncommitted_asset_state.version;
        let asset = match uncommitted_asset_state.result {
            UpdateAssetResult::Result(asset) => asset,
            UpdateAssetResult::AsyncResult(rx) => rx.recv_timeout(std::time::Duration::from_secs(0))
                .expect("LoadOp committed but result not sent via channel"),
        };

        // If a load handler exists, trigger the commit_asset_version callback
        self.load_handler.commit_asset_version(
            load_handle,
            version,
        );

        let asset_state = AssetState {
            asset,
            asset_uuid,
            version
        };

        // Commit the result
        self.assets.insert(load_handle, asset_state);
    }

    fn free(
        &mut self,
        load_handle: LoadHandle,
    ) {
        // Remove it from the list of assets
        let asset_state = self.assets.remove(&load_handle);

        if let Some(asset_state) = asset_state {
            log::trace!(
                "free {} {:?} {:?}",
                core::any::type_name::<LoadedT>(),
                load_handle,
                asset_state.asset_uuid
            );

            // Trigger the free callback on the load handler, if one exists
            self.load_handler.free(load_handle);
        }
    }

    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}
