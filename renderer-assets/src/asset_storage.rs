use atelier_assets::loader::{
    crossbeam_channel::Sender,
    handle::{AssetHandle, RefOp, TypedAssetStorage},
    AssetLoadOp, AssetStorage, AssetTypeId, LoadHandle, LoaderInfoProvider, TypeUuid,
};
use mopa::{mopafy, Any};
use std::{sync::Mutex, collections::HashMap, error::Error, sync::Arc};

use atelier_assets::importer as atelier_importer;
use atelier_assets::loader as atelier_loader;
use atelier_assets::core::AssetUuid;
use std::marker::PhantomData;

// Used to catch asset changes and upload them to the GPU (or some other system)
pub trait ResourceLoadHandler<T>: 'static + Send
where
    T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send,
{
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        version: u32,
        asset: &T,
        load_op: AssetLoadOp,
    );

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        asset_uuid: &AssetUuid,
        version: u32,
        asset: &T,
    );

    fn free(
        &mut self,
        load_handle: LoadHandle,
        version: u32,
    );
}

// Used to dynamic dispatch into a storage, supports checked downcasting
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

// Contains a storage per asset type
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

    pub fn add_storage<T>(&self)
    where
        T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send,
    {
        let mut storages = self.storage.lock().unwrap();
        storages.insert(
            AssetTypeId(T::UUID),
            Box::new(Storage::<T>::new(self.refop_sender.clone(), None)),
        );
    }

    //TODO: This could be redesigned to integrate better with the contents of asset_storage.rs
    // - Currently, we make a Storage<T> with T being the raw asset, and proxy events to the load handler
    // - The load handler goes through load queues and puts the instantiated asset in the resource
    //   manager's asset_lookup
    // - This means the asset storage has a fairly useless asset, and the actually useful struct
    //   (i.e. LoadedMesh vs. MeshAsset) has to be retrieved through a totally different path
    pub fn add_storage_with_load_handler<T, U>(
        &self,
        load_handler: Box<U>,
    ) where
        T: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send,
        U: ResourceLoadHandler<T>,
    {
        let mut storages = self.storage.lock().unwrap();
        storages.insert(
            AssetTypeId(T::UUID),
            Box::new(Storage::<T>::new(
                self.refop_sender.clone(),
                Some(load_handler),
            )),
        );
    }
}

// Implement atelier's AssetStorage - an untyped trait that finds the asset_type's storage and
// forwards the call
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

// Implement atelier's TypedAssetStorage - a typed trait that finds the asset_type's storage and
// forwards the call
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

struct AssetState<A> {
    version: u32,
    asset_uuid: AssetUuid,
    asset: A,
}

// A strongly typed storage for a single asset type
pub struct Storage<A: TypeUuid> {
    refop_sender: Arc<Sender<RefOp>>,
    assets: HashMap<LoadHandle, AssetState<A>>,
    uncommitted: HashMap<LoadHandle, AssetState<A>>,
    load_handler: Option<Box<dyn ResourceLoadHandler<A>>>,
}
impl<A: TypeUuid> Storage<A> {
    fn new(
        sender: Arc<Sender<RefOp>>,
        load_handler: Option<Box<dyn ResourceLoadHandler<A>>>,
    ) -> Self {
        Self {
            refop_sender: sender,
            assets: HashMap::new(),
            uncommitted: HashMap::new(),
            load_handler,
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

impl<A: for<'a> serde::Deserialize<'a> + 'static + TypeUuid + Send> TypedStorage for Storage<A> {
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
            core::any::type_name::<A>(),
            load_handle,
            loader_info.get_asset_id(load_handle).unwrap(),
            version
        );

        // To enable automatic serde of Handle, we need to set up a SerdeContext with a RefOp sender
        let asset = atelier_loader::handle::SerdeContext::with_sync(
            loader_info,
            self.refop_sender.clone(),
            || bincode::deserialize::<A>(data),
        )?;

        let asset_uuid = loader_info.get_asset_id(load_handle).unwrap();

        // Add to list of uncommitted assets
        self.uncommitted.insert(
            load_handle,
            AssetState {
                asset_uuid,
                asset,
                version,
            },
        );

        // If we have a load handler, fire the update_asset callback, otherwise trigger
        // load_op.complete() immediately
        if let Some(load_handler) = &mut self.load_handler {
            // We have a load handler, pass it a reference to the asset and a load_op. The load handler
            // will be responsible for calling load_op.complete() or load_op.error()
            let asset_state = self.uncommitted.get(&load_handle).unwrap();
            load_handler.update_asset(
                load_handle,
                &asset_uuid,
                version,
                &asset_state.asset,
                load_op,
            );
        } else {
            // Since there is no load handler, we call load_op.complete() immediately
            load_op.complete();
        }

        Ok(())
    }

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
        version: u32,
    ) {
        // Remove from the uncommitted list
        let asset_state = self
            .uncommitted
            .remove(&load_handle)
            .expect("asset not present when committing");

        log::trace!(
            "commit_asset_version {} {:?} {:?} {}",
            core::any::type_name::<A>(),
            load_handle,
            asset_state.asset_uuid,
            version
        );

        // If a load handler exists, trigger the commit_asset_version callback
        if let Some(load_handler) = &mut self.load_handler {
            load_handler.commit_asset_version(
                load_handle,
                &asset_state.asset_uuid,
                version,
                &asset_state.asset,
            );
        }

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
                core::any::type_name::<A>(),
                load_handle,
                asset_state.asset_uuid
            );

            // Trigger the free callback on the load handler, if one exists
            if let Some(load_handler) = &mut self.load_handler {
                load_handler.free(load_handle, asset_state.version);
            }
        }
    }

    fn type_name(&self) -> &'static str {
        core::any::type_name::<Self>()
    }
}
