//TODO: Add a render resource builder or something that lets someone add extract-only things by
// ref and resources available to all jobs by passing ownership

use super::AssetManager;
use std::ops::Deref;

// static reference is dangerous, must only be used when extracting
pub struct AssetManagerRenderResource(&'static AssetManager);

impl AssetManagerRenderResource {
    pub unsafe fn new(world: &AssetManager) -> Self {
        AssetManagerRenderResource(force_to_static_lifetime(world))
    }
}

impl Deref for AssetManagerRenderResource {
    type Target = AssetManager;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

unsafe fn force_to_static_lifetime<T>(value: &T) -> &'static T {
    std::mem::transmute(value)
}