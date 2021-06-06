//TODO: Add a render resource builder or something that lets someone add extract-only things by
// ref and resources available to all jobs by passing ownership

use super::AssetManager;
use rafx_base::memory::force_to_static_lifetime;
use std::ops::Deref;

// static reference is dangerous, must only be used when extracting. This is an option and is unset
// while not extracting.
#[derive(Default)]
pub struct AssetManagerRenderResource(Option<&'static AssetManager>);

impl AssetManagerRenderResource {
    pub unsafe fn set_asset_manager(
        &mut self,
        asset_manager: Option<&AssetManager>,
    ) {
        self.0 = asset_manager.map(|x| force_to_static_lifetime(x));
    }
}

impl Deref for AssetManagerRenderResource {
    type Target = AssetManager;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("AssetManager only available to render thread during extract")
    }
}
