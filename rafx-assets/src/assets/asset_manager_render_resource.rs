use super::AssetManager;
use rafx_base::memory::force_to_static_lifetime;
use std::ops::Deref;
use std::sync::Arc;

/// static reference is dangerous, must only be used when extracting. This is an option and is unset
/// while not extracting.
#[derive(Default)]
pub struct AssetManagerRenderResource(Option<Arc<&'static AssetManager>>);

impl AssetManagerRenderResource {
    /// Sets the AssetManager ref
    pub unsafe fn begin_extract(
        &mut self,
        asset_manager: &AssetManager,
    ) {
        assert!(self.0.is_none());
        self.0 = Some(Arc::new(force_to_static_lifetime(asset_manager)));
    }

    /// Clears the AssetManager ref and panics if any extract ref is remaining
    pub fn end_extract(&mut self) {
        self.0
            .take()
            .to_owned()
            .expect("Reference to AssetManager is still in use");
    }

    /// Returns an extract ref. Panics if called while not extracting. The ref must be dropped
    /// before end_extract() is called.
    pub fn extract_ref(&self) -> AssetManagerExtractRef {
        AssetManagerExtractRef(self.0.as_ref().unwrap().clone())
    }
}

/// "Borrowed" from AssetManagerRenderResource, must be dropped before extract ends
#[derive(Clone)]
pub struct AssetManagerExtractRef(Arc<&'static AssetManager>);

impl Deref for AssetManagerExtractRef {
    type Target = AssetManager;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
