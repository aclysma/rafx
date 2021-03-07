use crate::distill_impl::{AssetResource, ResourceAssetLoader};
use crate::{AssetLookup, AssetManager, DynAssetLookup, LoadQueues};
use crossbeam_channel::Sender;
use distill::loader::storage::AssetLoadOp;
use rafx_api::RafxResult;
use std::any::TypeId;
use std::marker::PhantomData;
use type_uuid::TypeUuid;

pub trait AssetTypeHandlerFactory {
    /// Register the asset type into the asset resource
    fn create(asset_resource: &mut AssetResource) -> Box<dyn AssetTypeHandler>;
}

pub trait AssetTypeHandler: Sync + Send {
    /// Called every frame to process load queues
    fn process_load_requests(
        &mut self,
        asset_manager: &mut AssetManager,
    ) -> RafxResult<()>;

    /// Return the asset lookup which can be used to obtain the committed and the latest (but
    /// possibly not committed) version of the asset
    fn asset_lookup(&self) -> &dyn DynAssetLookup;

    /// Returns the TypeId of the asset
    fn asset_type_id(&self) -> TypeId;
}

//
// A default asset type handler implementation for asset types that can implement a simple "load"
// function to convert from asset data to the asset
//
pub trait DefaultAssetTypeLoadHandler<AssetDataT, AssetT> {
    fn load(
        asset_manager: &mut AssetManager,
        font_asset: AssetDataT,
    ) -> RafxResult<AssetT>;
}

pub struct DefaultAssetTypeHandler<AssetDataT, AssetT, LoadHandlerT>
where
    LoadHandlerT: DefaultAssetTypeLoadHandler<AssetDataT, AssetT>,
{
    asset_lookup: AssetLookup<AssetT>,
    load_queues: LoadQueues<AssetDataT, AssetT>,
    phantom_data: PhantomData<LoadHandlerT>,
}

impl<AssetDataT, AssetT, LoadHandlerT> AssetTypeHandlerFactory
    for DefaultAssetTypeHandler<AssetDataT, AssetT, LoadHandlerT>
where
    AssetDataT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
    AssetT: TypeUuid + 'static + Send + Clone + Sync,
    LoadHandlerT: DefaultAssetTypeLoadHandler<AssetDataT, AssetT> + 'static + Sync + Send,
{
    fn create(asset_resource: &mut AssetResource) -> Box<dyn AssetTypeHandler> {
        let load_queues = LoadQueues::<AssetDataT, AssetT>::default();

        asset_resource.add_storage_with_loader::<AssetDataT, AssetT, _>(Box::new(
            ResourceAssetLoader(load_queues.create_loader()),
        ));

        Box::new(Self {
            asset_lookup: AssetLookup::new(asset_resource.loader()),
            load_queues,
            phantom_data: Default::default(),
        })
    }
}

impl<AssetDataT, AssetT, LoadHandlerT> AssetTypeHandler
    for DefaultAssetTypeHandler<AssetDataT, AssetT, LoadHandlerT>
where
    AssetDataT: TypeUuid + for<'a> serde::Deserialize<'a> + 'static + Send + Clone,
    AssetT: TypeUuid + 'static + Send + Clone + Sync,
    LoadHandlerT: DefaultAssetTypeLoadHandler<AssetDataT, AssetT> + 'static + Sync + Send,
{
    fn process_load_requests(
        &mut self,
        asset_manager: &mut AssetManager,
    ) -> RafxResult<()> {
        for request in self.load_queues.take_load_requests() {
            log::trace!(
                "Create asset type {} {:?}",
                std::any::type_name::<AssetT>(),
                request.load_handle
            );
            let loaded_asset = LoadHandlerT::load(asset_manager, request.asset);
            handle_load_result(
                request.load_op,
                loaded_asset,
                &mut self.asset_lookup,
                request.result_tx,
            );
        }

        handle_commit_requests(&mut self.load_queues, &mut self.asset_lookup);
        handle_free_requests(&mut self.load_queues, &mut self.asset_lookup);
        Ok(())
    }

    fn asset_lookup(&self) -> &dyn DynAssetLookup {
        &self.asset_lookup
    }

    fn asset_type_id(&self) -> TypeId {
        TypeId::of::<AssetT>()
    }
}

//
// For use with assets where the load data can be used as the asset directly
//
#[derive(Default)]
pub struct StorageOnlyAssetTypeLoadHandler<AssetT>(PhantomData<AssetT>);

impl<AssetT> DefaultAssetTypeLoadHandler<AssetT, AssetT>
    for StorageOnlyAssetTypeLoadHandler<AssetT>
{
    fn load(
        _asset_manager: &mut AssetManager,
        font_asset: AssetT,
    ) -> RafxResult<AssetT> {
        Ok(font_asset)
    }
}

pub type StorageOnlyAssetTypeHandler<AssetT> =
    DefaultAssetTypeHandler<AssetT, AssetT, StorageOnlyAssetTypeLoadHandler<AssetT>>;

//
// Static functions
//
pub fn handle_load_result<AssetT: Clone>(
    load_op: AssetLoadOp,
    loaded_asset: RafxResult<AssetT>,
    asset_lookup: &mut AssetLookup<AssetT>,
    result_tx: Sender<AssetT>,
) {
    match loaded_asset {
        Ok(loaded_asset) => {
            asset_lookup.set_uncommitted(load_op.load_handle(), loaded_asset.clone());
            result_tx.send(loaded_asset).unwrap();
            load_op.complete()
        }
        Err(err) => {
            load_op.error(err);
        }
    }
}

pub fn handle_commit_requests<AssetDataT, AssetT>(
    load_queues: &mut LoadQueues<AssetDataT, AssetT>,
    asset_lookup: &mut AssetLookup<AssetT>,
) {
    for request in load_queues.take_commit_requests() {
        log::trace!(
            "commit asset {:?} {}",
            request.load_handle,
            core::any::type_name::<AssetDataT>()
        );
        asset_lookup.commit(request.load_handle);
    }
}

pub fn handle_free_requests<AssetDataT, AssetT>(
    load_queues: &mut LoadQueues<AssetDataT, AssetT>,
    asset_lookup: &mut AssetLookup<AssetT>,
) {
    for request in load_queues.take_commit_requests() {
        asset_lookup.commit(request.load_handle);
    }
}
