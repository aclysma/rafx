use hydrate_base::LoadHandle;
use hydrate_loader::storage::ArtifactLoadOp;

use crossbeam_channel::Receiver;

pub struct RafxResourceLoadResult<T>
where
    T: 'static + Send,
{
    pub result_rx: Receiver<T>,
}

impl<T> RafxResourceLoadResult<T>
where
    T: 'static + Send,
{
    pub fn new(result_rx: Receiver<T>) -> Self {
        RafxResourceLoadResult { result_rx }
    }
}

// Used to catch asset changes and upload them to the GPU (or some other system)
pub trait RafxLoadEventHandler<AssetDataT, AssetT>: 'static + Send
where
    AssetDataT: for<'a> serde::Deserialize<'a>,
    AssetT: 'static + Send,
{
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        load_op: ArtifactLoadOp,
        asset: AssetDataT,
    ) -> RafxResourceLoadResult<AssetT>;

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
    );

    fn free(
        &mut self,
        load_handle: LoadHandle,
    );
}
