use renderer::assets::ResourceLoader;
use renderer::assets::GenericLoader;
use atelier_assets::loader::AssetLoadOp;
use atelier_assets::loader::LoadHandle;
use atelier_assets::loader::LoaderInfoProvider;
use atelier_assets::loader::handle::SerdeContext;
use atelier_assets::loader::handle::RefOp;
use type_uuid::TypeUuid;
use crate::asset_storage::{DynAssetLoader, UpdateAssetResult};
use crossbeam_channel::Sender;
use std::error::Error;

pub struct ResourceAssetLoader<AssetDataT, AssetT>(pub GenericLoader<AssetDataT, AssetT>)
where
    AssetDataT: for<'a> serde::Deserialize<'a> + 'static + Send,
    AssetT: TypeUuid + 'static + Send;

impl<AssetDataT, AssetT> DynAssetLoader<AssetT> for ResourceAssetLoader<AssetDataT, AssetT>
where
    AssetDataT: for<'a> serde::Deserialize<'a> + 'static + Send,
    AssetT: TypeUuid + 'static + Send,
{
    fn update_asset(
        &mut self,
        refop_sender: &Sender<RefOp>,
        loader_info: &dyn LoaderInfoProvider,
        data: &[u8],
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        _version: u32,
    ) -> Result<UpdateAssetResult<AssetT>, Box<dyn Error>> {
        // To enable automatic serde of Handle, we need to set up a SerdeContext with a RefOp sender
        let asset = futures_lite::future::block_on(SerdeContext::with(
            loader_info,
            refop_sender.clone(),
            async { bincode::deserialize::<AssetDataT>(data) },
        ))?;

        let result = self.0.update_asset(load_handle, load_op, asset);
        Ok(UpdateAssetResult::AsyncResult(result.result_rx))
    }

    fn commit_asset_version(
        &mut self,
        handle: LoadHandle,
        _version: u32,
    ) {
        self.0.commit_asset_version(handle);
    }

    fn free(
        &mut self,
        handle: LoadHandle,
    ) {
        self.0.free(handle);
    }
}
