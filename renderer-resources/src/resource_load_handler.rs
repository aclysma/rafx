use atelier_assets::loader::{AssetLoadOp, LoadHandle};

use crossbeam_channel::Receiver;

pub struct ResourceLoadResult<T>
    where
        T: 'static + Send,
{
    pub result_rx: Receiver<T>
}

impl<T> ResourceLoadResult<T>
    where
        T: 'static + Send,
{
    pub fn new(result_rx: Receiver<T>) -> Self {
        ResourceLoadResult {
            result_rx
        }
    }
}

// Used to catch asset changes and upload them to the GPU (or some other system)
pub trait ResourceLoadHandler<AssetT, LoadedT>: 'static + Send
where
    AssetT: for<'a> serde::Deserialize<'a>,
    LoadedT: 'static + Send,
{
    fn update_asset(
        &mut self,
        load_handle: LoadHandle,
        load_op: AssetLoadOp,
        asset: AssetT,
    ) -> ResourceLoadResult<LoadedT>;

    fn commit_asset_version(
        &mut self,
        load_handle: LoadHandle,
    );

    fn free(
        &mut self,
        load_handle: LoadHandle,
    );
}
