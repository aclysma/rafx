use downcast_rs::Downcast;
use fnv::FnvHashMap;
use hydrate_base::handle::ResolvedLoadHandle;
use hydrate_base::LoadHandle;
use hydrate_loader::loader::Loader;
use std::sync::Arc;

//
// Represents a single asset which may simultaneously have committed and uncommitted loaded state
//
pub struct LoadedAssetState<AssetT> {
    pub committed: Option<AssetT>,
    pub uncommitted: Option<AssetT>,
}

impl<AssetT> Default for LoadedAssetState<AssetT> {
    fn default() -> Self {
        LoadedAssetState {
            committed: None,
            uncommitted: None,
        }
    }
}

pub trait DynAssetLookup: Downcast {}

downcast_rs::impl_downcast!(DynAssetLookup);

pub struct AssetLookup<AssetT> {
    //TODO: Slab these for faster lookup?
    pub loaded_assets: FnvHashMap<LoadHandle, LoadedAssetState<AssetT>>,
}

impl<AssetT> DynAssetLookup for AssetLookup<AssetT> where AssetT: 'static {}

impl<AssetT> AssetLookup<AssetT> {
    pub fn new(loader: &Loader) -> Self {
        AssetLookup {
            loaded_assets: Default::default(),
        }
    }

    pub fn set_uncommitted(
        &mut self,
        load_handle: LoadHandle,
        loaded_asset: AssetT,
    ) {
        log::trace!("set_uncommitted {:?}", load_handle);
        debug_assert!(!load_handle.is_indirect());
        self.loaded_assets
            .entry(load_handle)
            .or_default()
            .uncommitted = Some(loaded_asset);
    }

    pub fn commit(
        &mut self,
        load_handle: LoadHandle,
    ) {
        log::trace!("commit {:?}", load_handle);
        debug_assert!(!load_handle.is_indirect());
        let state = self.loaded_assets.get_mut(&load_handle).unwrap();
        state.committed = state.uncommitted.take();
    }

    pub fn free(
        &mut self,
        load_handle: LoadHandle,
    ) {
        log::trace!("free {:?}", load_handle);
        debug_assert!(!load_handle.is_indirect());
        let old = self.loaded_assets.remove(&load_handle);
        assert!(old.is_some());
    }

    pub fn get_latest(
        &self,
        resolved_load_handle: &Arc<ResolvedLoadHandle>,
    ) -> Option<&AssetT> {
        if let Some(loaded_assets) = self
            .loaded_assets
            .get(&resolved_load_handle.direct_load_handle())
        {
            if let Some(uncommitted) = &loaded_assets.uncommitted {
                Some(uncommitted)
            } else if let Some(committed) = &loaded_assets.committed {
                Some(committed)
            } else {
                // It's an error to reach here because of uncommitted and committed are none, there
                // shouldn't be an entry in loaded_assets
                unreachable!();
            }
        } else {
            None
        }
    }

    pub fn get_committed(
        &self,
        resolved_load_handle: &Arc<ResolvedLoadHandle>,
    ) -> Option<&AssetT> {
        if let Some(loaded_assets) = self
            .loaded_assets
            .get(&resolved_load_handle.direct_load_handle())
        {
            if let Some(committed) = &loaded_assets.committed {
                Some(committed)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.loaded_assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.loaded_assets.is_empty()
    }

    pub fn destroy(&mut self) {
        self.loaded_assets.clear();
    }
}
