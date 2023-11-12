pub use hydrate_loader::asset_storage::{DynAssetLoader, UpdateAssetResult};
pub use hydrate_loader::AssetManager as AssetResource;

use crate::load_queue_hydrate::RafxGenericLoadEventHandler;
use crate::resource_loader::RafxLoadEventHandler;
use crossbeam_channel::Sender;
use hydrate_base::handle::LoaderInfoProvider;
use hydrate_base::handle::RefOp;
use hydrate_base::handle::SerdeContext;
use hydrate_base::LoadHandle;
use hydrate_loader::storage::AssetLoadOp;
use std::error::Error;
use type_uuid::TypeUuid;

//TODO: Seems like we can remove asset_resource and asset_storage so we just
// have ResourceAssetLoader, rename the file to resource_asset_loader and move up.

pub struct RafxResourceAssetLoader<AssetDataT, AssetT>(
    pub RafxGenericLoadEventHandler<AssetDataT, AssetT>,
)
where
    AssetDataT: for<'a> serde::Deserialize<'a> + 'static + Send,
    AssetT: TypeUuid + 'static + Send;

impl<AssetDataT, AssetT> DynAssetLoader<AssetT> for RafxResourceAssetLoader<AssetDataT, AssetT>
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
    ) -> Result<UpdateAssetResult<AssetT>, Box<dyn Error + Send>> {
        // To enable automatic serde of Handle, we need to set up a SerdeContext with a RefOp sender
        let asset = SerdeContext::with(loader_info, refop_sender.clone(), || {
            bincode::deserialize::<AssetDataT>(data)
                // Coerce into boxed error
                .map_err(|x| -> Box<dyn Error + Send + 'static> { Box::new(x) })
        })?;

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
