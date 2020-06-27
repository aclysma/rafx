use atelier_assets::loader::{AssetLoadOp, LoadHandle, TypeUuid};

use atelier_assets::core::AssetUuid;

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
